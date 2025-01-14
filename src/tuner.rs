use log::{debug, error, warn, info};
use signal_hook::{consts::SIGPIPE, consts::SIGINT, consts::SIGTERM, 
    consts::SIGUSR1, consts::SIGUSR2 ,iterator::Signals};
use std::env;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Write, BufReader, BufWriter};
use std::io::prelude::*;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process;
use std::result::Result;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::thread;
use std::time::{Duration, SystemTime};

nix::ioctl_write_buf!(set_ch, 0x8d, 0x01,IoctlFreq);
nix::ioctl_none!(start_rec, 0x8d, 0x02);
nix::ioctl_none!(stop_rec, 0x8d, 0x03);
nix::ioctl_read!(ptx_get_cnr, 0x8d, 0x04, i64);
nix::ioctl_write_int!(ptx_enable_lnb, 0x8d, 0x05);
nix::ioctl_none!(ptx_disable_lnb, 0x8d, 0x06);
nix::ioctl_write_int!(ptx_set_sys_mode, 0x8d, 0x0b);

use crate::arib_b25::{ARIB_STD_B25, ARIB_STD_B25_BUFFER, B_CAS_CARD};
use crate::commands::{CommanLineOpt, DecoderOptions, PROGRAM_RECPT};
//use crate::commands::TRUE;
use crate::decoder::{b25_startup, b25_decode, b25_shutdown};
use crate::ts_splitter_core::{LENGTH_PACKET, MAX_PID, get_pid, split_startup, split_select, split_ts,
    TSS_ERROR, TSS_SUCCESS};

// BSデバイスファイル名
const BSDEV: [&str; 92] = [
    "/dev/pt1video1",
    "/dev/pt1video0",
    "/dev/pt1video5",
    "/dev/pt1video4",
    "/dev/pt1video9",
    "/dev/pt1video8",
    "/dev/pt1video13",
    "/dev/pt1video12",
    "/dev/pt3video1",
    "/dev/pt3video0",
    "/dev/pt3video5",
    "/dev/pt3video4",
    "/dev/pt3video9",
    "/dev/pt3video8",
    "/dev/pt3video13",
    "/dev/pt3video12",
    "/dev/px4video0",
    "/dev/px4video1",
    "/dev/px4video4",
    "/dev/px4video5",
    "/dev/px4video8",
    "/dev/px4video9",
    "/dev/px4video12",
    "/dev/px4video13",
    "/dev/asv52201",
    "/dev/asv52200",
    "/dev/asv52205",
    "/dev/asv52204",
    "/dev/asv52209",
    "/dev/asv52208",
    "/dev/asv522013",
    "/dev/asv522012",
    "/dev/pxq3pe0",
    "/dev/pxq3pe1",
    "/dev/pxq3pe4",
    "/dev/pxq3pe5",
    "/dev/pxq3pe8",
    "/dev/pxq3pe9",
    "/dev/pxq3pe12",
    "/dev/pxq3pe13",
    "/dev/pxw3u30",
    "/dev/pxw3u32",
    "/dev/pxs3u20",
    "/dev/pxs3u0",

    "/dev/px4-DTV0",
    "/dev/px4-DTV1",
    "/dev/px4-DTV4",
    "/dev/px4-DTV5",
    "/dev/px4-DTV8",
    "/dev/px4-DTV9",
    "/dev/px4-DTV12",
    "/dev/px4-DTV13",
    "/dev/px4-DTV16",
    "/dev/px4-DTV17",
    "/dev/px4-DTV20",
    "/dev/px4-DTV21",
    "/dev/px4-DTV24",
    "/dev/px4-DTV25",
    "/dev/px4-DTV28",
    "/dev/px4-DTV29",

    "/dev/px5-DTV0",
    "/dev/px5-DTV1",
    "/dev/px5-DTV2",
    "/dev/px5-DTV3",
    "/dev/px5-DTV4",
    "/dev/px5-DTV5",
    "/dev/px5-DTV6",
    "/dev/px5-DTV7",
    "/dev/px5-DTV8",
    "/dev/px5-DTV9",
    "/dev/px5-DTV10",
    "/dev/px5-DTV11",
    "/dev/px5-DTV12",
    "/dev/px5-DTV13",
    "/dev/px5-DTV14",
    "/dev/px5-DTV15",
    "/dev/px5-DTV16",
    "/dev/px5-DTV17",
    "/dev/px5-DTV18",
    "/dev/px5-DTV19",
    "/dev/px5-DTV20",
    "/dev/px5-DTV21",
    "/dev/px5-DTV22",
    "/dev/px5-DTV23",
    "/dev/px5-DTV24",
    "/dev/px5-DTV25",
    "/dev/px5-DTV26",
    "/dev/px5-DTV27",
    "/dev/px5-DTV28",
    "/dev/px5-DTV29",
    "/dev/px5-DTV30",
    "/dev/px5-DTV31",
];

// 地上波デバイスファイル名
const ISDB_T_DEV: [&str; 92] = [
    "/dev/pt1video2",
    "/dev/pt1video3",
    "/dev/pt1video6",
    "/dev/pt1video7",
    "/dev/pt1video10",
    "/dev/pt1video11",
    "/dev/pt1video14",
    "/dev/pt1video15",
    "/dev/pt3video2",
    "/dev/pt3video3",
    "/dev/pt3video6",
    "/dev/pt3video7",
    "/dev/pt3video10",
    "/dev/pt3video11",
    "/dev/pt3video14",
    "/dev/pt3video15",
    "/dev/px4video2",
    "/dev/px4video3",
    "/dev/px4video6",
    "/dev/px4video7",
    "/dev/px4video10",
    "/dev/px4video11",
    "/dev/px4video14",
    "/dev/px4video15",
    "/dev/asv52202",
    "/dev/asv52203",
    "/dev/asv52206",
    "/dev/asv52207",
    "/dev/asv522010",
    "/dev/asv522011",
    "/dev/asv522014",
    "/dev/asv522015",
    "/dev/pxq3pe2",
    "/dev/pxq3pe3",
    "/dev/pxq3pe6",
    "/dev/pxq3pe7",
    "/dev/pxq3pe10",
    "/dev/pxq3pe11",
    "/dev/pxq3pe14",
    "/dev/pxq3pe15",
    "/dev/pxw3u31",
    "/dev/pxw3u33",
    "/dev/pxs3u21",
    "/dev/pxs3u1",

    "/dev/px4-DTV2",
    "/dev/px4-DTV3",
    "/dev/px4-DTV6",
    "/dev/px4-DTV7",
    "/dev/px4-DTV10",
    "/dev/px4-DTV11",
    "/dev/px4-DTV14",
    "/dev/px4-DTV15",
    "/dev/px4-DTV18",
    "/dev/px4-DTV19",
    "/dev/px4-DTV22",
    "/dev/px4-DTV23",
    "/dev/px4-DTV26",
    "/dev/px4-DTV27",
    "/dev/px4-DTV30",
    "/dev/px4-DTV31",

    "/dev/px5-DTV0",
    "/dev/px5-DTV1",
    "/dev/px5-DTV2",
    "/dev/px5-DTV3",
    "/dev/px5-DTV4",
    "/dev/px5-DTV5",
    "/dev/px5-DTV6",
    "/dev/px5-DTV7",
    "/dev/px5-DTV8",
    "/dev/px5-DTV9",
    "/dev/px5-DTV10",
    "/dev/px5-DTV11",
    "/dev/px5-DTV12",
    "/dev/px5-DTV13",
    "/dev/px5-DTV14",
    "/dev/px5-DTV15",
    "/dev/px5-DTV16",
    "/dev/px5-DTV17",
    "/dev/px5-DTV18",
    "/dev/px5-DTV19",
    "/dev/px5-DTV20",
    "/dev/px5-DTV21",
    "/dev/px5-DTV22",
    "/dev/px5-DTV23",
    "/dev/px5-DTV24",
    "/dev/px5-DTV25",
    "/dev/px5-DTV26",
    "/dev/px5-DTV27",
    "/dev/px5-DTV28",
    "/dev/px5-DTV29",
    "/dev/px5-DTV30",
    "/dev/px5-DTV31",
];

// ファイルバッファサイズ設定
#[allow(dead_code)]
pub const CAP: usize = 188 * 87;

// ioctl freq 
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct IoctlFreq {
    ch: i32,
    slot: i32,
}

#[derive(Debug, Clone)]
struct BsChannel {
    channel: i32,
    set_freq: i32,
    slot: i32,
}

// BSチャンネル一覧
const BS_CHANNELS: [BsChannel; 31] = [
    BsChannel{channel: 151, set_freq:  0, slot: 0}, // 151ch：BS朝日
    BsChannel{channel: 161, set_freq:  0, slot: 1}, // 161ch：BS-TBS
    BsChannel{channel: 171, set_freq:  0, slot: 2}, // 171ch：BSテレ東
    BsChannel{channel: 191, set_freq:  1, slot: 0}, // 191ch：WOWOWプライム
    BsChannel{channel: 103, set_freq:  1, slot: 1}, // 103ch：NHKBSプレミアム
    BsChannel{channel: 192, set_freq:  2, slot: 0}, // 192ch：WOWOWライブ
    BsChannel{channel: 193, set_freq:  2, slot: 1}, // 193ch：WOWOWシネマ
    BsChannel{channel: 211, set_freq:  4, slot: 0}, // 211ch：BS11イレブン
    BsChannel{channel: 200, set_freq:  4, slot: 1}, // 200ch：スター・チャンネル1
    BsChannel{channel: 222, set_freq:  4, slot: 2}, // 222ch：BS12トゥエルビ
    BsChannel{channel: 141, set_freq:  6, slot: 0}, // 141ch：BS日テレS
    BsChannel{channel: 181, set_freq:  6, slot: 1}, // 181ch：BSフジ
    BsChannel{channel: 236, set_freq:  6, slot: 2}, // 236ch：BSアニマックス
    BsChannel{channel: 231, set_freq:  6, slot: 2}, // 231ch：放送大学キャンパスex
    BsChannel{channel: 232, set_freq:  6, slot: 2}, // 232ch：放送大学キャンパスon
    BsChannel{channel: 233, set_freq:  6, slot: 2}, // 531ch：放送大学ラジオ
    BsChannel{channel: 101, set_freq:  7, slot: 0}, // 101ch：NHKBS1
    BsChannel{channel: 102, set_freq:  7, slot: 0}, // 102ch：NHKBS1
    BsChannel{channel: 201, set_freq:  7, slot: 1}, // 201ch：スター・チャンネル2
    BsChannel{channel: 202, set_freq:  7, slot: 1}, // 202ch：スター・チャンネル3
    BsChannel{channel: 245, set_freq:  9, slot: 0}, // 245ch：J SPORTS 4
    BsChannel{channel: 242, set_freq:  9, slot: 1}, // 242ch：J SPORTS 1
    BsChannel{channel: 243, set_freq:  9, slot: 2}, // 243ch：J SPORTS 2
    BsChannel{channel: 244, set_freq:  9, slot: 3}, // 244ch：J SPORTS 3
    BsChannel{channel: 252, set_freq: 10, slot: 0}, // 252ch：WOWOWプラス
    BsChannel{channel: 255, set_freq: 10, slot: 1}, // 255ch：日本映画専門ch
    BsChannel{channel: 234, set_freq: 10, slot: 2}, // 234ch：グリーンチャンネル
    BsChannel{channel: 256, set_freq: 11, slot: 0}, // 256ch：ディズニーch
    BsChannel{channel: 265, set_freq: 11, slot: 1}, // 265ch：BSよしもと
    BsChannel{channel: 263, set_freq: 11, slot: 2}, // 263ch：BSJapanext
    BsChannel{channel: 260, set_freq: 11, slot: 3}, // 260ch：BS松竹東急
];

// Show Channel Function
#[allow(dead_code)]
pub(crate)fn show_channels() {

    // Homeディレクトリーの設定
    let home: &str = env!("HOME");

    // チャンネルファイル名の設定
    let channel_file: String = format!("{}/.{}-channels", home, PROGRAM_RECPT);    

    // チャンネルファイル名のリード
    match fs::read_to_string(channel_file) {
        Ok(content) => eprintln!("{}", content),
        Err(_) => eprintln!("13-62: Terrestrial Channels"),
    }

    // チャンネルリストの表示
    eprintln!("BS01_0: BS朝日");
    eprintln!("BS01_1: BS-TB");
    eprintln!("BS01_2: BSテレ東");
    eprintln!("BS03_0: WOWOWプライム");
    eprintln!("BS13_1: BSアニマックス");
    eprintln!("BS13_2: BS釣りビジョン");
    eprintln!("BS05_0: WOWOWライブ");
    eprintln!("BS05_1: WOWOWシネマ");
    eprintln!("BS09_0: BS11イレブン");
    eprintln!("BS09_1: スターチャンネル1");
    eprintln!("BS09_2: BS12トゥエルビ");
    eprintln!("BS13_0: BS日テレ");
    eprintln!("BS13_1: BSフジ");
    eprintln!("BS13_2: 放送大学");
    eprintln!("BS15_0: NHKBS1");
    eprintln!("BS15_1: スターチャンネル2/3");
    eprintln!("BS19_0: J SPORTS 4");
    eprintln!("BS19_1: J SPORTS 1");
    eprintln!("BS19_2: J SPORTS 2");
    eprintln!("BS19_3: J SPORTS 3");
    eprintln!("BS21_0: WOWOWプラス");
    eprintln!("BS21_1: 日本映画専門ch");
    eprintln!("BS21_2: グリーンチャンネル");
    eprintln!("BS23_0: ディズニーch");
    eprintln!("BS23_1: BSよしもと");
    eprintln!("BS23_2: BSJapanext");
    eprintln!("BS23_3: BS松竹東急");
    eprintln!("C13-C63: CATV Channels");
    eprintln!("CS2-CS24: CS Channels");

}

// チャンネルタイプの判定処理
pub fn channel_type(channel: String) -> (String, IoctlFreq) {

    // channel_type 変数の作成と初期化
    let mut channel_type: String = "".to_string();
    let mut channel_num: i32 = 0;
    let mut slot_num: i32 = 0;

    // BSタイプの判定
    if channel.to_uppercase().starts_with("BS") {
        let channel_info = channel.to_uppercase().to_string();
        let channel_info: Vec<&str> = channel_info.split('_').collect();
        debug!("channel_type channel_info = {:?}",channel_info);

        if (1..=23).contains(&channel_info[0].replace("BS","").parse::<i32>().unwrap()) {
            channel_type = "BS".to_string();
            channel_num = channel_info[0].replace("BS","").parse::<i32>().unwrap() / 2;
            slot_num = channel_info[1].parse::<i32>().unwrap();
        }
    }

    // CSタイプの判定
    if channel.to_uppercase().starts_with("CS") {
        let channel_info = channel.to_uppercase().to_string();

        if (2..=24).contains(&channel_info.replace("CS","").parse::<i32>().unwrap()) {
            channel_type = "CS".to_string();
            channel_num = channel_info.replace("CS","").parse::<i32>().unwrap() / 2 + 11;
            slot_num = 0;
        }
    }

    // CATVタイプの判定
    if channel.to_uppercase().chars().nth(0).unwrap() == 'C' && 
       matches!(channel.to_uppercase().chars().nth(1).unwrap(),'0'..='9') {

        let channel_info = channel.to_uppercase().to_string();
        //let channel_info: Vec<&str> = channel_info.split('_').collect();

        (channel_type, channel_num) = match channel_info.replace("C","").parse::<i32>().unwrap() {
            13..=22 => {
                ("CATV".to_string(), channel_info.replace("C","").parse::<i32>().unwrap() - 10)
            },
            23..=63 => {
                ("CATV".to_string(), channel_info.replace("C","").parse::<i32>().unwrap() - 1)
            },
            _ => ("".to_string(), 0),
        };
        slot_num = 0;

    }

    // 地上波タイプ/BSチャンネルの判定
    match channel.parse::<i32>() {
        Ok(n) => {
            if (13..=62).contains(&n) {
                channel_type = "T".to_string();
                channel_num = channel.parse::<i32>().unwrap() + 50;
                slot_num = 0;
            }
            else {
                // BSチャンネル検索
                match BS_CHANNELS.iter()
                        .find(|ch| ch.channel == channel.parse::<i32>().unwrap())
                        .map(|ch| (ch.set_freq, ch.slot)) {
                    Some((lnb,slot)) => {
                        channel_type = "BS".to_string();
                        channel_num = lnb;
                        slot_num = slot;
                    },
                    None => {
                        channel_type = "".to_string();
                        channel_num = 0;
                        slot_num = 0;
                    },
                };
            };
        },
        Err(_) => { }
    };

    debug!("channel_type channel type,num,slot = {},{},{}", channel_type, channel_num, slot_num);
    // リターン情報
    (channel_type, IoctlFreq {ch: channel_num, slot: slot_num})

}

// 電波シグナルの受信処理
pub fn signal_get(device_file: &File, channel_type: &String) -> f32 {

    // シグナルの変数設定
    let mut signal_rc: i64 = 0;

    // 電波シグナルの受信
    unsafe { ptx_get_cnr(device_file.as_raw_fd(), &mut signal_rc).unwrap(); };

    // 電波シグナルの計算
    let signal: f32 = match &channel_type[..] {
        // 地上波の計算
        "CATV" | "T" => {
            let p = (5505024.0 / signal_rc as f32).log10() * 10.0;
            (0.000024 * p * p * p * p) - (0.0016 * p * p * p) +
            (0.0398 * p * p) + (0.5491 * p)+3.0965
        },
        // 地上波以外の計算
        _ => {
            //const AF_LEVEL_TABLE: [f32; 14] = [
            const AF_LEVEL_TABLE: [f32; 14] = [
                24.07, // 00    00    0        24.07dB
                24.07, // 10    00    4096     24.07dB
                18.61, // 20    00    8192     18.61dB
                15.21, // 30    00    12288    15.21dB
                12.50, // 40    00    16384    12.50dB
                10.19, // 50    00    20480    10.19dB
                8.140, // 60    00    24576    8.140dB
                6.270, // 70    00    28672    6.270dB
                4.550, // 80    00    32768    4.550dB
                3.730, // 88    00    34816    3.730dB
                3.630, // 88    FF    35071    3.630dB
                2.940, // 90    00    36864    2.940dB
                1.420, // A0    00    40960    1.420dB
                0.000, // B0    00    45056    -0.01dB
                ];

                let sig = (((signal_rc & 0xff00) >> 8) ) as u8;

                if sig <= 0x10u8 {
                    24.07
                }
                else if sig >= 0xb0u8 {
                    0.0
                }
                else {
                    let f_mixrate = (((sig as u16 & 0x0f) << 8) | sig as u16) as f32 / 4096.0;
                    AF_LEVEL_TABLE[(sig >> 4) as usize] * (1.0 - f_mixrate)
                        + AF_LEVEL_TABLE[(sig >> 4) as usize + 0x01] * f_mixrate
                }
        },
    };

    // リターン情報
    signal
}


// 録画処理
#[allow(dead_code)]
pub fn recording(command_opt: &mut CommanLineOpt, decoder_opt: DecoderOptions) -> () {

    // 録画時間変数
    let mut rec_time: u64;

    // プロセスIDの表示
    info!("pid = {}", process::id());

    // チャンネル情報からチャンネルタイプ,チャンネル番号,Slot番号の設定
    let (channel_type, _freq) = channel_type(command_opt.channel.to_string());
    if channel_type == "" { 
        warn!("Bad Channel !!");
        return
    };

    // チューナーデバイスの検索
    let (device, file) = tuner_device(&command_opt.device, &command_opt.channel);

    // チューナーデバイスが見つからない場合はリターン
    if device == "" {
        warn!("No devices available");
        return;
    };

    // チューナーデバイスファイルの作成
    let device_file = file.unwrap();

    // チューナーの設定処理を呼び出し
    tune(&device, &device_file, &command_opt.channel, &command_opt._lnb);

    let signal = signal_get(&device_file, &channel_type);
    info!("C/N = {:.6} dB", signal);

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

    // BCAS初期化エラー時はB25デコードしない
    if result < 0  && command_opt.use_b25 == true {
        command_opt.use_b25 = false;
        error!("Disabled B25...");
    };

    // ts splitterの初期化処理
    let mut sp = split_startup(&command_opt.sid_list);
    let mut split_select_finish = TSS_ERROR;

    // 出力ファイルの作成＆オープン
    let mut outfile = BufWriter::with_capacity(CAP, File::create(command_opt.outfile.to_string()).unwrap());

    // 録画開始時刻の取得
    let start_time = SystemTime::now();

    // 録画開始コマンド出力
    unsafe { start_rec(device_file.as_raw_fd()).unwrap() };
    info!("Recording...");

    // 出力用のバッファ作成
    let mut data_reader = BufReader::with_capacity(CAP, &device_file);

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
                    debug!("recording SIGUSR1 received. cleaning up...");
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

    // パケットドロップチェック用のパケット巡回カウンター変数の作成
    let mut continuity_counter: i32;
    let mut continuity_counter_flag: [i32; MAX_PID] = [0; MAX_PID];
    let mut next_continuity_counter: [i32; MAX_PID] = [0; MAX_PID];

    // データレシーブカウンター初期化
    let mut rcount = 0;

    // 録画ストップコマンド実行フラグ
    let mut stop_command_flag = 0;

    // 録画ループ（録画時間が経過するまでループ）
    rec_time = {
        loop {

            // バッファへ読み込み、ファイル出力
            let length = {
                let read_buffer = data_reader.fill_buf().unwrap();

                // use_splitterがfalseの場合に対象PID取得処理をここで実施
                if command_opt.use_splitter == false {
                    // パケットドロップチェック用のデータバッファ作成
                    let mut check_data_buff: Vec<u8> = vec![0; read_buffer.len()];
                    check_data_buff[..read_buffer.len()].copy_from_slice(&read_buffer[..read_buffer.len()]);
                    if split_select_finish != TSS_SUCCESS {
                        split_select_finish = split_select(&mut sp, &mut check_data_buff);
                    }
                };

                // データレシーブカウンターアップ
                rcount += 1;

                // データ長がCAPと違う場合はデバッグ出力
                if read_buffer.len() != CAP {

                    debug!("録画終了？ (CAP={} , read_buffer.len={})", CAP, read_buffer.len());

                };

                // パケットドロップチェック用のバッファ処理インデックス
                let mut index = 0;

                // バッファ終了までループ(パケットドロップチェック)
                while (read_buffer.len() as i32 - index as i32 - LENGTH_PACKET as i32) >= 0 {

                    // PID取得
                    let pid = get_pid(&read_buffer[index..index + LENGTH_PACKET - 1]) as usize;

                    // パケット巡回カウンターの作成
                    continuity_counter = (read_buffer[index + 3] & 0x0f) as i32;

                    // パケットドロップチェック
                    if (sp.pmt_pids[pid] > 0 || (sp.pids[pid] > 0 && pid < 0x100)) &&
                        continuity_counter_flag[pid] == 1 && continuity_counter != next_continuity_counter[pid] {

                        // signal取得
                        let signal = signal_get(&device_file, &channel_type);
                        debug!("パケットドロップ PID={}(0x{:04x}) , continuity_counter={} , next_continuity_counter={} , rcount={} , signel={}",
                            pid, pid, continuity_counter, next_continuity_counter[pid], rcount, signal);

                    };

                    // 次パケット巡回カウンターの計算
                    next_continuity_counter[pid] = (continuity_counter + 1) & 0x0f;
                    continuity_counter_flag[pid] = 1;

                    // 次パケットまでインデックス更新
                    index += LENGTH_PACKET;

                };

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
                let mut result = 0;

                // ts splitter処理
                if command_opt.use_splitter == true && len > 0 {

                    // buffer.lenチェック
                    if (buffer.len() % LENGTH_PACKET) != 0 {

                        warn!("buffer.len({})がLENGTH_PACKET({})の倍数ではありません。", buffer.len(), LENGTH_PACKET);

                    }

                    // データバッファ作成
                    let mut data_buff: Vec<u8> = vec![0; buffer.len()];
                    data_buff[..buffer.len()].copy_from_slice(&buffer[..buffer.len()]);

                    // 処理するsidの取得
                    if split_select_finish != TSS_SUCCESS {

                        split_select_finish = split_select(&mut sp, &mut data_buff);

                    };

                    // sid取得OK時の処理
                    if split_select_finish == TSS_SUCCESS {

                        // sid split処理
                        result = split_ts(&mut sp, &mut data_buff, &mut split_buff);

                    };

                };

                // ファイル出力処理
                if len > 0 || split_buff.len() > 0 {

                    // 出力バッファ作成
                    let write_buffer = if command_opt.use_splitter == true {
                        
                        if result != TSS_SUCCESS && split_buff.len() > 0 {
                            debug!("recording split_ts failed split_buff.len={}", split_buff.len());
                        }
                            
                        // ts splitter処理時のバッファ作成
                        let buffer: &[u8] = &split_buff;
                        //debug!("recording buffer.len={}", buffer.len());

                        // リターン情報
                        buffer

                    }
                    else {

                        // リターン情報
                        buffer

                    };

                    // ファイル出力
                    if write_buffer.len() > 0 {

                        outfile.write_all(write_buffer).unwrap();
                        //debug!("recording write_buffer.len={}", write_buffer.len());

                        // バッファーフラッシュしてファイルに書き込み
                        match outfile.flush() {
                            Ok(_) => { },
                            Err(_) => { error!("write_buf_file.flush Error"); },
                        };

                    }

                }

                // リターン情報
                read_buffer.len()
            };
            
            // リードバッファクリア
            data_reader.consume(length);

            // 録画時間が経過したらループ終了
            rec_time = SystemTime::now().duration_since(start_time).unwrap().as_secs();

            if rec_time > command_opt.duration {

                // 録画ストップしデータが読み込まなくなったら終了
                if stop_command_flag == 1  && length == 0 {
                    break rec_time;
                }

                // 録画終了コマンド出力
                if stop_command_flag == 0 {
                    debug!("call stop_rec");
                    unsafe { stop_rec(device_file.as_raw_fd()).unwrap() };
                };
                stop_command_flag = 1;

            };

            // SIGNAL受信
            if loop_exit.load(Ordering::Relaxed) == true { break rec_time; };
        }
    };

    // B-CASリーダーシャットダウン
    if command_opt.use_b25 == true {
        unsafe { b25_shutdown(dec, bcas) };
        info!("B25 shutdown");
    };

    // 録画終了情報出力
    info!("Recorded {}sec", rec_time);

}

// チューナデバイスファイルの確定処理
pub fn tuner_device(device: &String, channel: &String) -> (String, Result<fs::File, io::Error>) {

    // チューナデバイスファイルの変数設定
    let mut tuner_dev = "".to_string();

    // チャンネル情報からチャンネルタイプ,チャンネル番号,Slot番号の設定
    let (channel_type, _freq) = channel_type(channel.to_string());
    debug!("tuner_device channel_type = {}", channel_type.to_string());

    // チャンネルタイプからデバイステーブルの設定
    let tuner = match &channel_type[..] {
        // BS,CSのチャンネルテーブルを設定
        "BS" | "CS" => BSDEV,
        // 地上波,CATVのチャンネルテーブルを設定 
        "CATV" | "T" => ISDB_T_DEV,
        // デフォルト（地上波）のチャンネルテーブルを設定
        _ => ISDB_T_DEV
    };

    // デバイスファイルが設定済時の処理
    if device == "" {

        // チャンネルテーブルの配列数分ループ
        for i in 0..tuner.len() {

            // チューナーデバイスファイルの存在チェック
            if Path::new(tuner[i]).exists() {
                debug!("tuner_device dev={}",tuner[i]);

                // チューナーデバイスファイルのオープン
                tuner_dev = tuner[i].to_string();

                match  OpenOptions::new().read(true).open(&tuner_dev) {

                    // オープンOKの場合はファイルディスクリプターをリターン
                    Ok(file) => {
                        // ファイルシステムSync(複数プロセスの同時オープン対応)
                        match file.sync_all() {
                            // エラーなしの場合
                            Ok(file) => file,
                            // エラーの場合は10ミリ秒スリープ
                            Err(_err) => {std::thread::sleep(Duration::from_secs_f64(0.01).into())},
                        };

                        file
                    },
                    // オープンNGの場合はループを再開
                    Err(_err) => {
                        drop(tuner_dev.to_string());
                        debug!("tuner_device {} not open continue", tuner_dev);
                        continue
                    },
                };

                // ループから抜ける
                break;

            }
        }
    }
    // デバイスファイルが設定未済時の処理
    else {
        // チューナーデバイスにコマンド指定デバイスを設定
        tuner_dev = device.clone();
    }

    let file = match File::open(&tuner_dev) {
        Ok(file) => file,
        Err(e) => return (tuner_dev, Err(e)),
    };

    // リターン情報
    (tuner_dev, Ok(file))

}

// チューナー設定
#[allow(dead_code)]
pub fn tune(device: &String, file: &File, channel: &String, lnb: &u64) -> () {

    // チューナーデバイスのファイルディスクリプタ作成
    let fd = file.as_raw_fd();
    debug!("tune tuner fd = {}", fd);

    // チャンネル情報からチャンネルタイプ,チャンネル番号,Slot番号の設定
    let (channel_type, freq) = channel_type(channel.to_string());
    debug!("tune node = {} , slot = {}", freq.ch, freq.slot);

    // LNB設定処理
    let errno = match lnb {
        // 地上波、CATV以外の場合に設定可能
        1 | 2 if channel_type != "T" || channel_type != "CATV" => {
            let errno = unsafe { ptx_enable_lnb(fd, *lnb).unwrap() };
            errno
        },
        // 全てで設定可能
        0 => {
            let errno = unsafe { ptx_disable_lnb(fd).unwrap() };
            errno
        },
        _ => { 0 },
    };
    if errno < 0 { warn!("Power on LNB failed: {}", device) };
    debug!("tune Freq = {},{}", freq.ch, freq.slot);

    // チャンネル設定
    let errno = unsafe { set_ch(fd,&[freq]).unwrap() };
    if errno < 0 { eprintln!("Cannot tune to the specified channel: {}", device) };
    info!("device = {}", device);

}
