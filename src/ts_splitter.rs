extern crate getopts;

use chrono::Local;
use colored::*;
use std::env;
use env_logger::{Builder, Env};
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, debug, info, warn};
use getopts::Options;
use signal_hook::{consts::SIGPIPE, consts::SIGINT, consts::SIGTERM,
    consts::SIGUSR1, consts::SIGUSR2 ,iterator::Signals};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, Write};
use std::process;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::thread;

mod arib_b25;
mod commands;
mod decoder;
mod ffi;
mod ts_splitter_core;
mod tuner;

use crate::arib_b25::{ARIB_STD_B25, ARIB_STD_B25_BUFFER, B_CAS_CARD};
use crate::commands::{PROGRAM_TS_SPLITTER, TRUE, FALSE};
use crate::commands::{CommanLineOpt, DecoderOptions};
use crate::decoder::{b25_startup, b25_decode, b25_shutdown};
use crate::ts_splitter_core::{split_startup, split_select, split_ts, TSS_SUCCESS};
use crate::tuner::CAP;

pub const VERSION: &str = env!("VERSION_TS_SPLITTER");

// Usage出力
pub fn show_usage(program: &str, opts: &Options) {

    let brief = format!("Usage: {} --sid SID1,SID2,... infile ouitfile", program);
    eprintln!("{}", opts.usage(&brief));

}

pub(crate) fn command_line_check(program: &str) -> (CommanLineOpt, DecoderOptions) {

    let mut use_b25: bool = false;
    //let mut use_bell: bool = false;
    //let mut use_udp: bool = false;
    let use_http: bool = false;
    let http_port: u16 = 0;
    let mut use_splitter: bool = false;
    //let mut host_to: String = "".to_string();
    //let mut port_to: u16 = 0;
    let device: String = "".to_string();
    let mut sid_list: String = "".to_string();
    let use_round: bool = false;
    let use_lnb: bool = false;
    let lnb: u64 = 0;
    let use_device: bool = false;
    let channel: String = "".to_string();
    let duration: u64 = 0;
    let mut _infile: String = "".to_string();
    let mut _outfile: String = "".to_string();

    let mut dopt = DecoderOptions {
        round: 4,
        strip: FALSE,
        emm: FALSE,
    };

    // 実行時に与えられた引数をargs: Vec<String>に格納する
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();

    // オプションを設定
    opts.optflag("b","b25","Decrypt using BCAS card");
    opts.optflag("s","strip","Strip null stream");
    opts.optflag("m","EMM","Instruct EMM operation");
    opts.optflag("h","help","Show this help");
    opts.optopt("","sid","Specify SID number in CSV format (101,102,...)","SID1,SID2,...");
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

    // b25デコードの有効設定
    if matches.opt_present("b25") {
        use_b25 = true;
    };

    // b25デコードオプションフラグ(strip)の設定
    if matches.opt_present("strip") {
        dopt.strip = TRUE;
    };

    // b25デコードオプションフラグ(EMM)の設定
    if matches.opt_present("EMM") {
       dopt.emm= TRUE;
    };

    // 処理するSIDを設定
    if matches.opt_present("sid") {
        use_splitter = true;
        sid_list = matches.opt_str("sid").unwrap().to_string();
    };

    // 引数（オプションを除く）判定処理
    match matches.free.len() {
        // 2つでuse_splitterがtrueの場合は入力、出力ファイルを設定
        2 if use_splitter == true => {
            _infile = matches.free[0].clone();
            _outfile = matches.free[1].clone();
        },
        // 上記以外はヘルプを表示
        _ => {
            show_usage(&program, &mut &opts);
            process::exit(0);
        },
    };

    // リターン情報を設定
    (
        CommanLineOpt {
            _program: program.to_string(),
            use_b25: use_b25,
            //use_bell: use_bell,
            //use_udp: use_udp,
            _use_http: use_http,
            _http_port: http_port,
            //host_to: host_to.to_string(),
            //port_to: port_to,
            device: device.to_string(),
            sid_list: sid_list.to_string(),
            use_splitter: use_splitter,
            _use_round: use_round,
            _use_lnb: use_lnb,
            _lnb: lnb,
            _use_device: use_device,
            channel: channel,
            duration: duration,
            infile: _infile.to_string(),
            outfile: _outfile.to_string(),
        },
        DecoderOptions {
            round: dopt.round,
            strip: dopt.strip,
            emm: dopt.emm,
        }
    )
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
    .init();

    // コマンドラインオプションチェック
    let program = PROGRAM_TS_SPLITTER;
    let (mut opt, dopt) = command_line_check(program);

    if opt.use_b25 == true { info!("using B25...") };
    if dopt.strip == TRUE { info!("enable B25 strip") };
    if dopt.emm == TRUE { info!("enable B25 emm processing") };

    // ts splitte処理呼び出し
    if opt.use_splitter == true && opt.infile != "" && opt.outfile != "" {
        ts_split(&mut opt, &dopt);
    };

}

fn ts_split(command_opt: &mut CommanLineOpt, decoder_opt: &DecoderOptions) -> () {

    // B25デコード処理
    let (result, dec, bcas) = match  command_opt.use_b25 {
        // フラグがtrueの場合に初期設定
        true => {
            // 初期化処理呼び出し
            let (result, dec, bcas) = unsafe {
                b25_startup(decoder_opt.round, decoder_opt.strip, decoder_opt.emm)
            };
            (result, dec, bcas)
        },
        // true以外は変数初期化のみ
        _ => (-1, 0 as *mut ARIB_STD_B25, 0 as *mut B_CAS_CARD),
    };
    if result < 0  && command_opt.use_b25 == true {
        command_opt.use_b25 = false;
        error!("Disabled B25...");
    };

    // ts splitterの初期化処理
    let mut sp = split_startup(&command_opt.sid_list);

    // 入力ファイルオープン
    let file = File::open(command_opt.infile.to_string()).unwrap();
    let mut read_buf_file = BufReader::with_capacity(CAP, &file);
    let file_size = file.metadata().unwrap().len();
    info!("input file={} , size={}", command_opt.infile.to_string(),file_size);

    // 出力ファイルオープン
    let mut write_buf_file = BufWriter::new(File::create(command_opt.outfile.to_string()).unwrap());
    info!("output file = {}", command_opt.outfile.to_string());

    // SIGNAL処理用の変数設定
    let loop_exit = Arc::new(AtomicBool::new(false));
    let loop_exit2 = Arc::clone(&loop_exit);
    let mut signals = Signals::new([SIGPIPE, SIGINT, SIGTERM, SIGUSR1, SIGUSR2]).unwrap();

    // SIGNAL処理スレッド
    thread::spawn(move || {
        for sig in signals.forever() {
            eprintln!("Received signal {:?}", sig);
            match sig {
                SIGPIPE => {
                    warn!("\nSIGPIPE received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                SIGINT => {
                    warn!("\nSIGINT received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                SIGTERM => {
                    warn!("\nSIGTERM received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                SIGUSR1 => {
                    debug!("SIGUSR1 received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                SIGUSR2 => {
                    warn!("\nSIGUSR2 received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                _ => {},
            };
        }
    });

    let pb = ProgressBar::new(file_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) \n {msg}")
        .unwrap()
        .progress_chars("#>-"));

    loop {
        let length = {

            // ファイルリードしバッファーに格納
            let read_buffer = read_buf_file.fill_buf().unwrap();

            // リードバッファ作成
            let b25_buff = ARIB_STD_B25_BUFFER {
                data: read_buffer.as_ptr() as *mut u8,
                size: read_buffer.len() as u32,
            };

            // B25デコード処理
            let (buffer, len) = match command_opt.use_b25 {
                // フラグがtrueの場合にデコード
                true => {
                    // デコード処理
                    let (buffer, len) = unsafe { b25_decode(dec, &b25_buff) };

                    // リターン情報
                    (buffer, len)
                },
                // フラグがtrue以外の処理
                false => {
                    // リードデータをそのまま出力
                    let buffer = read_buffer as &[u8];

                    // リターン情報
                    (buffer, read_buffer.len() as i32)
                },
            };

            // ts splitter向け変数
            let mut split_buff: Vec<u8> = vec![];
            let mut _result = 0;

            // ts splitter処理
            if command_opt.use_splitter == true && len > 0 {

                // データバッファ作成
                let mut data_buff: Vec<u8> = vec![0; buffer.len()];
                data_buff[..buffer.len()].copy_from_slice(&buffer[..buffer.len()]);

                // 処理するsidの取得
                _result = split_select(&mut sp, &mut data_buff);

                // sid取得OK時の処理
                if _result == TSS_SUCCESS {

                    // sid split処理
                    _result = split_ts(&mut sp, &mut data_buff, &mut split_buff);

                };

                // ファイル出力処理
                if split_buff.len() > 0 {
                     write_buf_file.write_all(&split_buff as &[u8]).unwrap();
                }
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

        // SIGNAL受信
        if loop_exit.load(Ordering::Relaxed) == true {
            break
        };
    }

    pb.finish_with_message("正常終了しました。");
    // B-CASリーダーシャットダウン
    if command_opt.use_b25 == true {
        unsafe { b25_shutdown(dec, bcas) };
        info!("B25 shutdown");
    };
}
