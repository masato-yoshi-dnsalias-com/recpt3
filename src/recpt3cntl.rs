extern crate getopts;

use chrono::Local;
use colored::*;
use env_logger::{Builder, Env, Target};
use getopts::Options;
use log::info;
use std::env;
use std::io::Write;
use std::process;
use posix_mq::{Message, Name, Queue};

use crate::commands::{PROGRAM_RECPT,PROGRAM_RECPTCNTL};

mod commands;

pub const VERSION: &str = env!("VERSION_RECPT3CNTL");

// Usage出力
pub fn show_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} --pid Process ID --time 録画時間", program);
    eprintln!("{}", opts.usage(&brief));

}

// struct CommanLineOpt
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommanLineOpt {
    pub _program: String,
    pub process_id: String,
    pub rec_time: String
}

// コマンドラインオプションの判定処理
pub(crate) fn command_line_check(program: &str) -> CommanLineOpt {

    let mut process_id: String = "".to_string();
    let mut rec_time:   String = "".to_string();

    // 実行時に与えられた引数をargs: Vec<String>に格納する
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();

    // オプションを設定
    opts.optopt("p","pid","Process", "ID");
    opts.optopt("t","time","RecTime", "number");
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

    // 録画時間の設定
    if matches.opt_present("time") {
        rec_time = matches.opt_str("time").unwrap();
        info!("Rec Time = {}", rec_time);
    }

    // メッセージ送信先プロセスIDを設定
    if matches.opt_present("pid") {
        process_id = matches.opt_str("pid").unwrap();
        info!("Send ProcessID = {}", process_id);
    }

    // 必須オプションチェック
    if rec_time == "" || process_id == "" {
        show_usage(&program, &mut &opts);
        process::exit(0);
    };

    // リターン情報を設定
    CommanLineOpt {
        _program: program.to_string(),
        process_id: process_id,
        rec_time:   rec_time,
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
    let program = PROGRAM_RECPTCNTL;
    let command_opt = command_line_check(program);

    let mq_name_id = format!("{}{}_{}","/",PROGRAM_RECPT,command_opt.process_id);
    // posixメッセージキュー名の作成
    let mq_name = Name::new(&mq_name_id).unwrap();

    // posixメッセージキューのオープン
    let queue = Queue::open(mq_name).expect("posixメッセージキューのオープンエラー");
    info!("posixメッセージキューのオープン(/dev/mqueue{})", &mq_name_id);

    // メッセージ作成
    let message = Message {
        data: format!("time={}",command_opt.rec_time).as_bytes().to_vec(),
        priority: 0,
    };

    // posix message 送信
    queue.send(&message).expect("Failed to send");

}
