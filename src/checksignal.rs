extern crate getopts;

use chrono::Local;
use colored::*;
use env_logger::{Builder, Env, Target};
use getopts::Options;
use log::{info, error};
use signal_hook::{consts::SIGINT, consts::SIGTERM,
    consts::SIGUSR1, iterator::Signals};
use std::env;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::process;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};

mod arib_b25;
mod commands;
mod decoder;
mod ffi;
mod ts_splitter_core;
mod tuner;

use crate::commands::{PROGRAM_CHECKSIGNAL};
use crate::tuner::{channel_type, signal_get, show_channels, tuner_device};

pub const VERSION: &str = env!("VERSION_CHECKSIGNAL");

// Usage出力
pub fn show_usage(program: &str, opts: &Options) {

    let brief = format!("Usage: {} [--device devicefile] [--lnb voltage] channel", program);
    eprintln!("{}", opts.usage(&brief));

}

// struct CommanLineOpt
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommanLineOpt {
    pub _program: String,
    pub device: String,
    pub _use_lnb: bool,
    pub _lnb: u64,
    pub _use_device: bool,
    pub channel: String,
}

// コマンドラインオプションの判定処理
pub(crate) fn command_line_check(program: &str) -> CommanLineOpt {

    let mut device: String = "".to_string();
    let mut use_lnb: bool = false;
    let mut lnb: u64 = 0;
    let mut use_device: bool = false;
    let mut _channel: String = "".to_string();

    // 実行時に与えられた引数をargs: Vec<String>に格納する
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();

    // オプションを設定
    opts.optopt("d","device","Specify devicefile to use","devicefile");
    opts.optopt("n","lnb","Specify LNB voltage (0, 11, 15)","voltage");
    //opts.optflag("b","bell","Notify signal quality by bell");
    opts.optflag("h","help","Show this help");
    opts.optflag("v","version","Show version");
    opts.optflag("l","list","Show channel list");

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

    // チャンネルリストを表示
    if matches.opt_present("list") {
        show_channels();
        process::exit(0);
    }
    // チューナーデバイスを設定
    if matches.opt_present("device") {
        use_device = true;
        device = matches.opt_str("device").unwrap().to_string();
        info!("using device: {}", device);
    }

    // LNB voltageの設定
    if matches.opt_present("lnb") {
        use_lnb = true;
        let lnb_str = matches.opt_str("lnb").unwrap().to_string();
        lnb = match &*lnb_str {
            "11" => 1 ,
            "15" => 2 ,
            _  => 0 ,
        };
    };

    // 引数（オプションを除く）判定処理
    match matches.free.len() {
        // 1つの場合はチャンネルを設定
        1 => {
            _channel = matches.free[0].clone();
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
        device: device.to_string(),
        _use_lnb: use_lnb,
        _lnb: lnb,
        _use_device: use_device,
        channel: _channel,
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
    let program = PROGRAM_CHECKSIGNAL;
    let command_opt = command_line_check(program);

    // チャンネル情報からチャンネルタイプ,チャンネル番号,Slot番号の設定
    let (channel_type, freq) = channel_type(command_opt.channel.to_string());
    if channel_type == "" {
        error!("Bad Channel !!");
        return
    };

    // チューナーデバイスの検索
    let (device, file) = tuner_device(&command_opt.device, &command_opt.channel);

    // チューナーデバイスが見つからない場合はリターン
    if device == "" {
        error!("No devices available");
        return;
    };

    // チューナーデバイスファイルの作成
    let device_file = file.unwrap();

    // チューナーの設定処理を呼び出し
    //tune(device, &device_file, &command_opt.channel, &command_opt.lnb);

    // SIGNAL処理用の変数設定
    let loop_exit = Arc::new(AtomicBool::new(false));
    let loop_exit2 = Arc::clone(&loop_exit);
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGUSR1]).unwrap();

    // SIGNAL処理スレッド
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    eprintln!("\nSIGINT received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                SIGTERM => {
                    eprintln!("\nSIGTERM received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                SIGUSR1 => {
                    eprintln!("SIGPIPE received. cleaning up...");
                    loop_exit2.store(true, Ordering::Release);
                },
                _ => {},

            };
        };
    });

    // チューナーデバイスのファイルディスクリプタ作成
    let fd = device_file.as_raw_fd();

    // LNB設定処理
    let errno = match command_opt._lnb {
        // 地上波以外
        1 | 2 if channel_type != "T" => {
            let errno = unsafe { tuner::ptx_enable_lnb(fd, command_opt._lnb).unwrap() };
            errno
        },
        // 地上波の場合
        0 if channel_type != "T" => {
            let errno = unsafe { tuner::ptx_disable_lnb(fd).unwrap() };
            errno
        },
        _ => { 0 },
    };
    if errno < 0 { error!("Power on LNB failed: {}", device) };

    // チャンネル設定
    let errno = unsafe { tuner::set_ch(fd,&[freq]).unwrap() };
    if errno < 0 { error!("Cannot tune to the specified channel: {}", device) };

    info!("device = {}", device);

    // SIGNAL受信までループ
    loop {

        // 電波シグナル受信
        let signal = signal_get(&device_file, &channel_type);
        eprint!("\rC/N = {:.6} dB", signal);

        // 1秒スリープ
        std::thread::sleep(Duration::from_secs_f64(1.0).into());

        // ループ終了判定
        if loop_exit.load(Ordering::Relaxed) == true { break };

    };

    // 終了処理
    if channel_type != "T" {

        // LNBなし設定
        unsafe { tuner::ptx_disable_lnb(fd).unwrap() };

    };

}
