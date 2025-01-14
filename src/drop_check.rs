extern crate getopts;

use chrono::Local;
use colored::*;
use std::env;
use env_logger::{Builder, Env, Target};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, debug, warn};
use getopts::Options;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader};
use std::process;

mod arib_b25;
mod commands;
mod decoder;
mod ffi;
mod ts_splitter_core;
mod tuner;

use crate::commands::{PROGRAM_DROP_CHECK};
use crate::ts_splitter_core::{LENGTH_PACKET, MAX_PID, get_pid, split_select, split_startup,
    TSS_ERROR, TSS_SUCCESS};
use crate::tuner::CAP;

pub const VERSION: &str = env!("VERSION_TS_SPLITTER");

// Usage出力
pub fn show_usage(program: &str, opts: &Options) {

    let brief = format!("Usage: {} --sid SID1,SID2,... infile ouitfile", program);
    eprintln!("{}", opts.usage(&brief));

}

// struct CommanLineOpt
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommanLineOpt {
    pub _program: String,
    pub infile: String,
}

pub(crate) fn command_line_check(program: &str) -> CommanLineOpt {

    let mut _infile: String = "".to_string();

    // 実行時に与えられた引数をargs: Vec<String>に格納する
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();

    // オプションを設定
    opts.optflag("h","help","Show this help");
    opts.optflag("v","version","Show version");

    // 未定義のオプションを指定した場合にエラーメッセージを出力する
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(msg) => {
            eprintln!("Error: {}", msg.to_string());
            show_usage(&program, &opts);
            process::exit(0);
        }
    };

    // ヘルプを表示し終了
    if matches.opt_present("help") {
        show_usage(&program, &opts);
        process::exit(0);
    }

    // バージョンを表示し終了
    if matches.opt_present("version") {
        eprintln!("{} {}",program, VERSION);
        eprintln!("recorder command for PT1/2/3 digital tuner.");
        process::exit(0);
    }

    // 引数（オプションを除く）判定処理
    match matches.free.len() {
        // 入力ファイルを設定
        1 => {
            _infile = matches.free[0].clone();
        },
        // 上記以外はヘルプを表示
        _ => {
            show_usage(&program, &mut &opts);
            process::exit(0);
        },
    };

    // リターン情報を設定
    CommanLineOpt {
        _program: program.to_string(),
        infile: _infile.to_string(),
    }
}

// メイン処理
fn main() {

    Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let local_time = Local::now().format("%Y/%m/%d %H:%M:%S");
            let level = match record.level() {
                log::Level::Error => "ERROR  ".red(),
                log::Level::Warn  => "WARNING".yellow(),
                log::Level::Info  => "INFO   ".green(),
                log::Level::Debug => "DEBUG  ".cyan(),
                log::Level::Trace => "TRACE  ".blue(),
            };
            let pid = process::id();
            writeln!(
                buf,
                "[{}] {} {} [{}] {}",
                local_time,
                level,
                record.target(),
                pid,
                record.args(),
            )
        }
    )
    .target(Target::Stdout)  // 出力先をStdoutに変更
    .init();

    // コマンドラインオプションチェック
    let program = PROGRAM_DROP_CHECK;
    let mut opt = command_line_check(program);

    // ドロップチェック処理呼び出し
    if opt.infile != "" {
        drop_check(&mut opt);
    };

}

//
// ドロップチェック処理
//
fn drop_check(command_opt: &mut CommanLineOpt) -> () {

    // ts splitterの初期化処理
    let mut sp = split_startup("all");
    let mut split_select_finish = TSS_ERROR;

    // 入力ファイルオープン
    let file = File::open(command_opt.infile.to_string()).unwrap();
    let mut read_buf_file = BufReader::with_capacity(CAP, &file);
    let file_size = file.metadata().unwrap().len();
    info!("input file={} , size={}", command_opt.infile.to_string(),file_size);

    // プログレスバー処理の初期化
    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) \n {msg}")
        .unwrap()
        .progress_chars("#>-"));

    // パケット巡回カウンター変数の作成
    let mut continuity_counter: i32;
    let mut continuity_counter_flag: [i32; MAX_PID] = [0; MAX_PID];
    let mut next_continuity_counter: [i32; MAX_PID] = [0; MAX_PID];

    loop {
        let length = {

            // ファイルリードしバッファーに格納
            let read_buffer = read_buf_file.fill_buf().unwrap();

            // ファイル終端以外の処理
            if read_buffer.len() > 0 {

                // データバッファ作成
                let mut data_buff: Vec<u8> = vec![0; read_buffer.len()];
                data_buff[..read_buffer.len()].copy_from_slice(&read_buffer[..read_buffer.len()]);
                if split_select_finish != TSS_SUCCESS {
                    split_select_finish = split_select(&mut sp, &mut data_buff);
                };

                // バッファ処理インデックス
                let mut index = 0;

                // バッファ終了までループ
                while (data_buff.len() as i32 - index as i32 - LENGTH_PACKET as i32) >= 0 {

                    // PID取得
                    let pid = get_pid(&data_buff[index..index + LENGTH_PACKET - 1]) as usize;

                    // パケット巡回カウンターの作成
                    continuity_counter = (data_buff[index + 3] & 0x0f) as i32;

                    // パケットドロップチェック
                    if (sp.pmt_pids[pid] > 0 || (sp.pids[pid] > 0 && pid < 0x100)) &&
                        continuity_counter_flag[pid] == 1 && continuity_counter != next_continuity_counter[pid] {

                        warn!("パケットドロップ PID={}(0x{:04x}) , continuity_counter={} , next_continuity_counter={}\n",
                            pid, pid, continuity_counter, next_continuity_counter[pid]);

                    };

                    // 次パケット巡回カウンターの計算
                    next_continuity_counter[pid] = (continuity_counter + 1) & 0x0f;
                    continuity_counter_flag[pid] = 1;

                    // 次パケットまでインデックス更新
                    index += LENGTH_PACKET;

                };
            };

            // リターン情報
            read_buffer.len()

        };

        // ファイルの終わりで終了
        if length == 0 {
            break
        }

        // progress bar描画
        pb.inc(length.try_into().unwrap());

        // バッファクリア
        read_buf_file.consume(length);

    };

    // プログレスバー終了メッセージ
    pb.finish_with_message("正常終了しました。");

    // デバッグ情報
    for cnt in 0..MAX_PID {

        if sp.pmt_pids[cnt] != 0 {

            debug!("sp.pmt_pids[{}(0x{:04x})]={}", cnt, cnt, sp.pmt_pids[cnt])

        };

    };
}
