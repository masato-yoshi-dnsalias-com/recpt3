use log::{debug, error, warn, info};
use std::io::{BufReader};
use std::io::prelude::*;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::process;
use std::time::{Duration, SystemTime};

use crate::arib_b25::{ARIB_STD_B25, ARIB_STD_B25_BUFFER, B_CAS_CARD};
use crate::commands::{CommanLineOpt, DecoderOptions};
use crate::decoder::{b25_startup, b25_decode, b25_shutdown};
use crate::ts_splitter_core::{LENGTH_PACKET, MAX_PID, get_pid, read_ts, split_startup, split_select, split_ts, TSS_SUCCESS};
use crate::tuner;
use crate::tuner::{CAP, channel_type, signal_get, start_rec, tuner_device, tune};

pub fn http_daemon(command_opt: CommanLineOpt, decoder_opt: DecoderOptions) -> () {

    debug!("http_daemon run as a daemon..");

    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), command_opt._http_port);
    let _addr_v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0,0,0,0)), command_opt._http_port);
    let listener = TcpListener::bind(&addr).unwrap();

    // 開始時刻の取得
    let start_time = SystemTime::now();

    // 録画時間が設定されている場合はタイマースレッドを起動
    if command_opt.duration > 0 {
        std::thread::spawn(move || {
            loop {

                // 実行時間を取得
                let run_time = SystemTime::now().duration_since(start_time).unwrap().as_secs();

                // 実行時間が設定時間を経過したらプロセス終了
                if run_time > command_opt.duration {
                    process::exit(0)
                }
                // 実行時間が設定時間以下の場合は１秒スリープ
                else {
                    std::thread::sleep(Duration::from_secs_f64(1.0).into())
                };

            };
        });
    };

    // コネクション接続待ち
    for stream in listener.incoming() {

        match stream {
            // コネクション受信
            Ok(stream) => {

                // オプション情報のコピー
                let mut command_opt = command_opt.clone();
                let decoder_opt = decoder_opt.clone();

                // スレッド起動まで1秒スリープ(デバイスファイルの空きが1つの場合にエラーになるのを回避するため)
                std::thread::sleep(Duration::from_secs_f64(1.0).into());

                // コネクション受信スレッド起動
                std::thread::spawn(move || {
                    response_stream(&mut command_opt, &decoder_opt, stream);
                });

            },
            // コネクション受信エラー
            Err(e) => {
                error!("Error: {}", e);
            },
        };

    }
}

// コネクションレスポンス処理
fn response_stream(command_opt: &mut CommanLineOpt, decoder_opt: &DecoderOptions, mut stream: TcpStream)
    -> () {

    // リクエスト格納バッファ
    let mut req_buff = [0; 4096];

    // コネクション受信ループ
    loop {
        match stream.read(&mut req_buff) {
            Ok(n) => {
                    
                // コネクションクローズ
                if n == 0 {
                    info!("Connection closed");
                    break;
                };

                // リクエストを行毎に分解
                let request  = std::str::from_utf8(&req_buff[..n]).unwrap();
                let request_line: Vec<&str> = request.lines().collect();
                debug!("response_stream request={:?}", request_line);
                
                // 1行目をパーツ毎に分解
                let request_line = request_line[0];
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap();
                let path = parts.next().unwrap();
                let version = parts.next().unwrap();
                debug!("response_stream Method={} , Path={} , Version{}", method, path, version);

                // urlを分割
                let uri: Vec<&str> = path.split('/').collect();
                debug!("response_stream uri.len()={},{:?}",uri.len(), uri); 

                // urlからチャンネルとsidを取得
                let (channel, sid) = match uri.len() {
                    // チャンネルとsidが有る時
                    3 => {
                        let channel = String::from(uri[1]);
                        let sid = String::from(uri[2]);
                        info!("channel={},sid={}",channel, sid);
                        (channel, sid)
                    },
                    // 上記以外
                    _ => {
                        (String::from(""), String::from(""))
                    },
                };

                // httpヘッダーのレスポンス送信
                let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nCache-Control: no-cache\r\n\r\n";
                stream.write_all(response).unwrap();
                stream.flush().unwrap();

                // チャンネル情報からチャンネルタイプ,チャンネル番号,Slot番号の設定
                let (channel_type, _freq) = channel_type(channel.to_string());
                if channel_type == "" {
                    warn!("Bad Channel !!");
                    return;
                };

                // チューナーデバイスの検索
                //let (device, file) = tuner_device(&"".to_string(), &channel.to_string());
                let (device, file) = tuner_device(&command_opt.device, &channel.to_string());

                // チューナーデバイスが見つからない場合はリターン
                if device == "" {
                    warn!("No devices available");
                    return;
                };

                // チューナーデバイスファイルの作成
                let device_file = file.unwrap();

                // チューナーの設定処理を呼び出し
                tune(&device, &device_file, &channel.to_string(), &command_opt._lnb);

                let signal = tuner::signal_get(&device_file, &channel_type);
                info!("C/N = {:.6} dB", signal);

                // B25デコード処理
                let (result, dec, bcas) = match command_opt.use_b25 {
                    // フラグがtrueの場合に初期設定
                    true => {
                        // 初期化処理呼び出し
                        let (result, dec, bcas) = unsafe {
                            b25_startup(decoder_opt.round,
                                decoder_opt.strip, decoder_opt.emm)
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
                let mut sp = split_startup(&sid);
                if sid != "" { command_opt.use_splitter = true; };

                // 録画開始コマンド出力
                unsafe { start_rec(device_file.as_raw_fd()).unwrap() };
                info!("Recording...");

                // 出力用のバッファ作成
                let mut data_reader = BufReader::with_capacity(CAP, &device_file);

                // パケットドロップチェック用のパケット巡回カウンター変数の作成
                let mut continuity_counter: i32;
                let mut continuity_counter_flag: [i32; MAX_PID] = [0; MAX_PID];
                let mut next_continuity_counter: [i32; MAX_PID] = [0; MAX_PID];

                loop {
                    // バッファへ読み込み
                    let length = {
                        let read_buffer = data_reader.fill_buf().unwrap();
                        //debug!("response_stream data_reader={:?}",&read_buffer.len());

                        // パケットドロップチェック用のデータバッファ作成
                        let mut data_buff: Vec<u8> = vec![0; read_buffer.len()];
                        data_buff[..read_buffer.len()].copy_from_slice(&read_buffer[..read_buffer.len()]);
                        let _result = read_ts(&mut sp, &mut data_buff);

                        // パケットドロップチェック用のバッファ処理インデックス
                        let mut index = 0;

                        // バッファ終了までループ(パケットドロップチェック)
                        while index < read_buffer.len() {

                            // PID取得
                            let pid = get_pid(&read_buffer[index..index+LENGTH_PACKET - 1]) as usize;

                            // パケット巡回カウンターの作成
                            continuity_counter = (read_buffer[index + 3] & 0x0f) as i32;

                            // パケットドロップチェック
                            if sp.pmt_pids[pid] > 0 && continuity_counter_flag[pid] == 1 && continuity_counter != next_continuity_counter[pid] {

                                let signal = signal_get(&device_file, &channel_type);
                                debug!("パケットドロップ PID={}(0x{:04x}) , continuity_counter={} , next_continuity_counter={} signel={}",
                                    pid, pid, continuity_counter, next_continuity_counter[pid], signal);

                            };

                            // 次パケット巡回カウンターの計算
                            next_continuity_counter[pid] = (continuity_counter + 1) & 0x0f;
                            continuity_counter_flag[pid] = 1;

                            // 次パケットまでインデックス更新
                            index += 188;

                        };

                        // リードバッファ作成
                        let b25_buff = ARIB_STD_B25_BUFFER {
                            //data: read_buffer.clone().as_ptr() as *mut u8,
                            data: read_buffer.as_ptr() as *mut u8,
                            size: read_buffer.len() as u32,
                        };

                        // B25デコード処理
                        let (buffer, len) = match command_opt.use_b25 {
                            // フラグがtrueの場合にデコード
                            true => {
                                // デコード処理
                                let (buffer, len) = unsafe { b25_decode(dec, &b25_buff) };

                                (buffer, len)
                            },
                            // フラグがtrue以外の処理
                            false => {
                                // リードデータをそのまま出力
                                let buffer = read_buffer as &[u8];

                                (buffer, read_buffer.len() as i32)
                            },
                        };

                        // ts splitter向け変数
                        let mut split_buff: Vec<u8> = vec![];
                        let mut result = 0;

                        // ts splitter処理
                        if command_opt.use_splitter == true &&  len > 0 {

                            // データバッファ作成
                            let mut data_buff: Vec<u8> = vec![0; buffer.len()];
                            data_buff[..buffer.len()].copy_from_slice(&buffer[..buffer.len()]);

                            // 処理するsidの取得
                            result = split_select(&mut sp, &mut data_buff);

                            // sid取得OK時の処理
                            if result == TSS_SUCCESS {

                                // sid split処理
                                result = split_ts(&mut sp, &mut data_buff, &mut split_buff);

                            };

                        };

                        // データ送信
                        if len > 0  ||  split_buff.len() > 0 {
                            // 出力バッファ作成
                            let write_buffer = if command_opt.use_splitter == true {

                                if result != TSS_SUCCESS && split_buff.len() > 0 {
                                    debug!("response_stream split_ts failed");
                                };

                                // ts splitter処理時のバッファ作成
                                let buffer: &[u8] = &split_buff;

                                // リターン情報
                                buffer

                            }
                            else {

                                // リターン情報
                                buffer
                            };

                            // ストリーム出力
                            if write_buffer.len() > 0 {
                                match stream.write_all(write_buffer) {
                                    Ok(_) => {},
                                    Err(e) => {
                                        error!("Data Send Error({})",e);

                                        // B-CASリーダーシャットダウン
                                        if command_opt.use_b25 == true {
                                            unsafe { b25_shutdown(dec, bcas) };
                                            info!("B25 shutdown");
                                        };

                                        break;
                                    },
                                };
                            };
                        }

                        // リターン情報
                        read_buffer.len()

                    };

                    // リードバッファクリア
                    //debug!("response_stream clear length={:?}", length);
                    data_reader.consume(length.try_into().unwrap());

                };

            },
            Err(e) => {
                error!("Error2: {}", e);
                break;
            },
        }
    }
}
