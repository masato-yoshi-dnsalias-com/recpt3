extern crate getopts;

use chrono::Local;
use colored::*;
use env_logger::{Builder, Env};
use getopts::Options;
use log::info;
use std::env;
use std::io::Write;
use std::process;

// use crate::commands::{PROGRAM_RECPT, TRUE, FALSE, command_line_check, CommanLineOpt, DecoderOptions};
use crate::commands::{PROGRAM_RECPT, TRUE, FALSE, CommanLineOpt, DecoderOptions};

mod arib_b25;
mod commands;
mod decoder;
mod ffi;
mod http_daemon;
mod ts_splitter_core;
mod tuner;

use crate::http_daemon::http_daemon;
use crate::tuner::{recording, show_channels};


pub const VERSION: &str = env!("VERSION_RECPT3");

// Usage出力
pub fn show_usage(program: &str, opts: &Options) {

    let brief = format!("Usage: {} [--b25 [--round N] [--strip] [--EMM]] [--http portnumber] [--device devicefile] [--lnb voltage] [--sid SID1,SID2,...] channel rectime outfile", program);
    eprintln!("{}", opts.usage(&brief));

}

// コマンドラインオプションの判定処理
pub(crate) fn command_line_check(program: &str) -> (CommanLineOpt, DecoderOptions) {

    let mut use_b25: bool = false;
    //let mut use_bell: bool = false;
    //let mut use_udp: bool = false;
    let mut use_http: bool = false;
    let mut http_port: u16 = 0;
    let mut use_splitter: bool = false;
    //let mut host_to: String = "".to_string();
    //let mut port_to: u16 = 0;
    let mut device: String = "".to_string();
    let mut sid_list: String = "".to_string();
    let mut use_round: bool = false;
    let mut use_lnb: bool = false;
    let mut lnb: u64 = 0;
    let mut use_device: bool = false;
    let mut _channel: String = "".to_string();
    let mut duration: u64 = 0;
    let mut _infile: String = "".to_string();
    let mut outfile: String = "".to_string();

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
    opts.optopt("r","round","Specify round number","N");
    opts.optflag("s","strip","Strip null stream");
    opts.optflag("m","EMM","Instruct EMM operation");
    //opts.optflag("u","udp","Turn on udp broadcasting");
    opts.optopt("a","addr","Hostname or address to connect","hostname");
    opts.optopt("p","port","Port number to connect","portnumber");
    opts.optopt("H","http","Turn on http broadcasting (run as a daemon)","port number");
    opts.optopt("d","device","Specify devicefile to use","devicefile");
    opts.optopt("n","lnb","Specify LNB voltage (0, 11, 15)","voltage");
    opts.optopt("","sid","Specify SID number in CSV format (101,102,...)","SID1,SID2,...");
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

    //
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
        info!("LNB = {}",lnb);
    };

    // b25デコードの有効設定
    if matches.opt_present("b25") {
        use_b25 = true;
        info!("using B25...");
    };

    // b25デコードオプションフラグ(strip)の設定
    if matches.opt_present("strip") {
        dopt.strip = TRUE;
        info!("enable B25 strip");
    };

    // b25デコードオプションフラグ(EMM)の設定
    if matches.opt_present("EMM") {
       dopt.emm= TRUE;
       info!("enable B25 emm processing");
    };

    // 処理するSIDを設定
    if matches.opt_present("sid") {
        use_splitter = true;
        sid_list = matches.opt_str("sid").unwrap().to_string();
    };

    // HTTP Broadcastingの有効設定
    if matches.opt_present("http") {
        use_http = true;
        http_port = matches.opt_str("http").unwrap().parse::<u16>().unwrap_or(0);
        info!("creating a http daemon");
    };

    // 処理時のラウンド係数を設定
    if matches.opt_present("round") {
        use_round = true;
        dopt.round = matches.opt_str("round").unwrap().parse::<i32>().unwrap_or(0);
        info!("set round {:?}", dopt.round);
    };

    /*
    // UDP Broadcastingの有効設定
    if matches.opt_present("udp") {
        use_udp = true;
        info!("enable UDP broadcasting");
    };

    // UDP送信先のホストを設定
    if matches.opt_present("addr") {
        use_udp = true;
        host_to = matches.opt_str("addr").unwrap().to_string();
        info!("UDP destination address: {}", host_to);
    };

    // UDP送信先のポート番号を設定
    if matches.opt_present("port") {
        use_udp = true;
        port_to = matches.opt_str("port").unwrap().parse::<u16>().unwrap_or(0);
        if port_to == 0 {
            show_usage(&program, &opts);
            process::exit(0);
        }
        info!("UDP port: {}", port_to);
    };
    */

    // 引数（オプションを除く）判定処理
    match matches.free.len() {
    // 無しの場合
    0 if use_http == true => {
        _channel = "".to_string();
        duration = 0 as u64;
        outfile = "".to_string();
    },
    1 => {
        _channel = matches.free[0].clone();
    },
    3 => {
        _channel = matches.free[0].clone();
        duration = matches.free[1].parse::<u64>().unwrap_or(0);
        outfile = matches.free[2].clone();
    },
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
            channel: _channel,
            duration: duration,
            infile: _infile.to_string(),
            outfile: outfile.to_string(),
        },
        DecoderOptions {
            round: dopt.round,
            strip: dopt.strip,
            emm: dopt.emm,
        },
        //opts
    )
}

// メイン処理
fn main() {

    // env_logの初期化
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
    let program = PROGRAM_RECPT;
    let (mut opt, dopt) = command_line_check(program);

    /*
    if opt.use_b25 == true { info!("using B25...") };
    if dopt.strip == TRUE { info!("enable B25 strip") };
    if dopt.emm == TRUE { info!("enable B25 emm processing") };
    if opt._use_http == true { info!("creating a http daemon") };
    if opt._use_round == true { info!("set round {:?}", dopt.round) };
    //if opt.use_udp == true { info!("enable UDP broadcasting") };
    //if opt.host_to != "" { info!("UDP destination address: {}", opt.host_to) };
    //if opt.port_to > 0 { info!("UDP port: {}", opt.port_to) };
    if opt._use_device == true { info!("using device: {}", opt.device) };
    if opt._use_lnb == true { info!("LNB = {}",opt._lnb) };
    */

    //if opt.sids.len() > 0 { info!("{}", opt.sids) };

    // http daemon処理
    if opt._use_http {
        http_daemon(opt.clone(), dopt.clone());
    }

    // 録画ファイル作成処理
    if opt.channel != "" && opt.duration > 0 && opt.outfile != "" {
        recording(&mut opt, dopt);
    };

}

