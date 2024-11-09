use log::{debug, info};
use crc::{Crc, CRC_32_ISO_HDLC};

pub const MAX_PID: usize = 8192;
pub const MAX_SERVICES: usize = 50;
pub const LENGTH_PACKET: usize = 188;
pub const TSS_SUCCESS: i32 = 0;
pub const TSS_ERROR: i32 = -1;
pub const SECTION_CONTINUE: i32 = 1;
pub const LENGTH_PAT_HEADER: i32 = 12;

#[derive(Debug, Copy, Clone)]
pub struct PmtVersion {
    pid: [i16; MAX_SERVICES],
    version: [u8; MAX_SERVICES],
}

#[derive(Debug, Clone)]
pub struct Splitter {
    pids: [u16; MAX_PID],
    pmt_pids: [u16; MAX_PID],
    pat: [u8; LENGTH_PACKET],
    sid_list: String,
    pat_count: u8,
    pmt_retain: i32,
    pmt_counter: i32,
    pmt_version: PmtVersion,
    section_remain: [u16; MAX_PID],
    packet_seq: [u8; MAX_PID],
}

// Ts Splitterの初期設定
pub fn split_startup(sid: &str) -> Splitter {

    // 変数の初期設定
    let sp = Splitter {
        pids: [0; MAX_PID],
        pmt_pids: [0; MAX_PID],
        pat: [0xff; LENGTH_PACKET],
        sid_list: sid.to_string(),
        pat_count: 0,
        pmt_retain: -1,
        pmt_counter: 0,
        pmt_version: {
            PmtVersion {
                pid: [0; MAX_SERVICES],
                version: [0; MAX_SERVICES],
            }
        },
        section_remain: [0; MAX_PID],
        packet_seq: [0; MAX_PID],
    };

    sp

}

// ドロップするPIDの確定処理
pub fn split_select(mut sp: &mut Splitter, buff: &mut Vec<u8>) -> i32 {

    // TS解析
    let result = read_ts(&mut sp, buff);

    // リターン情報
    result

}

// TS 分離処理
pub fn split_ts(mut sp: &mut Splitter, buff: &mut [u8], split_buff: &mut Vec<u8>) -> i32 {

    let mut result = TSS_SUCCESS ;
    let length = buff.len();
    let mut in_index = 0;
    let mut version = 0;
    let mut _drop = 0;

    // バッファエンドまでループ
    while (length as i16 - in_index as i16 - LENGTH_PACKET as i16) >= 0 {

        // PID取得
        let pid = get_pid(&buff[in_index..in_index+LENGTH_PACKET - 1]);

        // PID判定
        match pid {

            // PAT処理
            0x0000 => {

                // 巡回カウンタカウントアップ
                if sp.pat_count == 0xff {
                    sp.pat_count = sp.pat[3];
                }
                else {
                    sp.pat_count += 1;
                    if sp.pat_count % 0x10 == 0 {
                        sp.pat_count -= 0x10;
                    }
                }
                sp.pat[3] = sp.pat_count;

                // Splitバッファーに新しいPATを作成
                for cnt in 0..=LENGTH_PACKET - 1 {
                    split_buff.push(sp.pat[cnt]);
                }
            },
            // PAT以外の処理
            _ => {

                if sp.pmt_pids[pid as usize] != 0 {

                    // PMT (PES開始インジケータ)
                    if (buff[in_index + 1] & 0x40) == 0x40 {

                        // バージョンチェック
                        for pmts in 0..=sp.pmt_retain - 1 {
                            if sp.pmt_version.pid[pmts as usize] == pid {
                                version = sp.pmt_version.version[pmts as usize];
                            }
                        }

                        if version != buff[in_index + 10] & 0x3e ||
                            sp.pmt_retain != sp.pmt_counter {
                            // 再チェック
                            result = rescan_pid(&mut sp, &buff[in_index..]);
                        }
                    }
                    else {
                        if sp.pmt_retain != sp.pmt_counter {
                            // 再チェック
                            result = rescan_pid(sp, &buff[in_index..]);
                        }
                    };
                };

                // sp.pids[pid]が「1」のパケットは残すパケット
                if sp.pids[pid as usize] != 0 {
                    for cnt in 0..=LENGTH_PACKET - 1 {
                        // Splitバッファー作成
                        split_buff.push(buff[cnt + in_index]);
                    }
                }
                else {
                    // ドロップカウンターカウントアップ
                    _drop += 1;
                };
            },
        }

        // ループカウンタをパケットサイズ数分カウントアップ
        in_index += LENGTH_PACKET;

    }

    // リターン情報
    result
}

// PIDの再スキャン
pub fn rescan_pid(mut sp: &mut Splitter, buff: &[u8]) -> i32 {

    let mut result = TSS_ERROR;

    // クリア処理
    if sp.pmt_counter == sp.pmt_retain {

        // sp.pmt_counterのクリア
        sp.pmt_counter = 0;

        // sp.section_remain,sp.packet_seqのクリア
        for cnt in 0..=MAX_PID - 1 {
            sp.section_remain[cnt] = 0;
            sp.packet_seq[cnt] = 0;
        };
        eprintln!("Rescan PID");

    };

    if TSS_SUCCESS ==  analyze_pmt(&mut sp, &buff[0..], 2) {
        sp.pmt_counter += 1;
    }

    if sp.pmt_counter == sp.pmt_retain {

        result = TSS_SUCCESS;

        // PIDカウンターの減算
        for cnt in 0..=MAX_PID - 1 {
            if sp.pids[cnt] > 0 {
                sp.pids[cnt] -= 1;
            };
        };
    };

    // リターン情報
    result

}

// TS 解析処理
// 対象のチャンネル番号のみの PAT の再構築と出力対象 PID の抽出を行う
pub fn read_ts(mut sp: &mut Splitter, data: &mut [u8]) -> i32 {

    let mut result = TSS_ERROR;

    // 変数初期化
    let length = data.len();
    let mut index = 0;

    // バッファエンドまでループ
    while (length as i16 - index as i16 - LENGTH_PACKET as i16) >= 0 {

        // PID取得
        let pid = get_pid(&data[index..index+LENGTH_PACKET - 1]);

        if pid == 0x0000 {
            // PAT解析処理
            result = analyze_pat(&mut sp, &data[index..index+LENGTH_PACKET - 1]);
            if result != TSS_SUCCESS {
                return result;
            }
        }

        // PMT
        // PID毎に初回のみPMT判定
        if sp.pmt_pids[pid as usize] == 1 {

            // PMT解析処理
            let analyze_result = analyze_pmt(&mut sp, &data[index..index+LENGTH_PACKET - 1], 1);

            // 判定正常終了時の処理
            if analyze_result == TSS_SUCCESS {
                sp.pmt_pids[pid as usize] += 1;
                sp.pmt_counter += 1;
                data[index + 1] = 0xff;
                data[index + 2] = 0xff;
            }

        }

        // 録画する全てのPMTについて、中にあるPCR/AUDIO/VIDEOのPIDを得る
        // pmt_counter と pmt_retain が一致する場合に条件は満たされる
        if sp.pmt_counter == sp.pmt_retain {
            result = TSS_SUCCESS;
            break;
        }
        else {
            result = TSS_ERROR;
        }

        // ループカウンタをパケットサイズ数分カウントアップ
        index += LENGTH_PACKET;

    }

    // リターン情報
    result

}

// PAT解析処理
pub fn analyze_pat(mut sp: &mut Splitter, data: &[u8]) -> i32 {

    // 変数設定
    let mut result = TSS_SUCCESS;
    let mut avail_sids: Vec<i16> = vec![];
    let mut chosen_sid: Vec<i16> = vec![];
    let mut avail_pmts: Vec<i16> = vec![];
    let mut pid_pos: Vec<usize> = vec![];
    let sid_list: Vec<&str> = sp.sid_list.split(',').collect();
    let sid_count = sid_list.len();

    let pat_section_size = ((data[6] as i16 & 0x0f) << 8) + data[7] as i16;
    let size = pat_section_size + 4;

    // 
    if sp.pat[0] == 0xff {

        let mut cnt: usize = 13.try_into().unwrap();

        sp.pmt_retain = 0;

        // prescan SID/PMT
        while (cnt + 4)  <= size.try_into().unwrap() {

            let index: usize = (cnt + 1 ).try_into().unwrap();
            let pid = get_pid(&data[index..]);
            if pid == 0x0010 {
                cnt += 4;
                continue
            };

            let service_id: i16 = ((data[cnt] as i16) << 8) + data[cnt + 1] as i16;
            avail_sids.push(service_id);
            avail_pmts.push(pid);
            debug!("service_id[{}] = {:x} , pid = {:x}", index, service_id, pid);

            cnt += 4;

        }

        // 対象チャンネル判定
        cnt = 13.try_into().unwrap();
        while (cnt + 4)  <= size.try_into().unwrap() {

            let index: usize = (cnt + 1 ).try_into().unwrap();
            
            // PAT
            let pid = get_pid(&data[index..]);
            if pid == 0x0010 {
                cnt += 4;
                continue
            };

            // サービスIDの取得
            let service_id: i16 = ((data[cnt] as i16) << 8) + data[cnt + 1] as i16;

            // prescanで取得したサービスID数分ループ
            for sid in &sid_list {
                
                // サービスID毎の設定処理
                match &*sid.to_uppercase() {

                    // HD,SD1
                    "HD" | "SD1" if service_id == avail_sids[0] => {
                        chosen_sid.push(service_id);
                        sp.pmt_pids[pid as usize] = 1;
                        sp.pids[pid as usize] = 1;
                        pid_pos.push(cnt);
                        sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                        sp.pmt_retain += 1;
                        debug!("pid={} , service_id={} \"hd or sd1\"", pid, service_id);
                    },

                    // SD2
                    "SD2" if service_id == avail_sids[1] => {
                        chosen_sid.push(service_id);
                        sp.pmt_pids[pid as usize] = 1;
                        sp.pids[pid as usize] = 1;
                        pid_pos.push(cnt);
                        sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                        sp.pmt_retain += 1;
                        debug!("pid={} , service_id={} \"sd2\"", pid, service_id);
                    },

                    // SD3
                    "SD3" if service_id == avail_sids[2] => {
                        chosen_sid.push(service_id);
                        sp.pmt_pids[pid as usize] = 1;
                        sp.pids[pid as usize] = 1;
                        pid_pos.push(cnt);
                        sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                        sp.pmt_retain += 1;
                        debug!("pid={} , service_id={} \"sd3\"", pid, service_id);
                    },

                    // 1SEG
                    "1SEG" if pid == 0x1FC8 => {
                        chosen_sid.push(service_id);
                        sp.pmt_pids[pid as usize] = 1;
                        sp.pids[pid as usize] = 1;
                        pid_pos.push(cnt);
                        sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                        sp.pmt_retain += 1;
                        debug!("pid={} , service_id={} \"1seg\"", pid, service_id);
                    },

                    // 全て
                    "ALL" => {
                        chosen_sid.push(service_id);
                        sp.pmt_pids[pid as usize] = 1;
                        sp.pids[pid as usize] = 1;
                        pid_pos.push(cnt);
                        sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                        sp.pmt_retain += 1;
                        debug!("pid={} , service_id={} \"all\"", pid, service_id);
                    },
                    
                    // EPG
                    "EPG" => {
                        sp.pids[0x11] = 1;
                        sp.pids[0x12] = 1;
                        sp.pids[0x23] = 1;
                        sp.pids[0x29] = 1;
                        debug!("pid={} , service_id={} \"epg\"", pid, service_id);
                    },

                    // EPG1SEG
                    "EPG1SEG" => {
                        sp.pids[0x11] = 1;
                        sp.pids[0x26] = 1;
                        sp.pids[0x27] = 1;
                        debug!("pid={} , service_id={} \"epg1seg\"", pid, service_id);
                    },

                    // その他
                    "" if sid_count == 1 => {
                        chosen_sid.push(service_id);
                        sp.pmt_pids[pid as usize] = 1;
                        sp.pids[pid as usize] = 1;
                        pid_pos.push(cnt);
                        sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                        sp.pmt_retain += 1;
                        debug!("pid={} , service_id={} \"\"", pid, service_id);
                    },
                    _ => {
                        match sid.parse::<i16>() {
                            Ok(sid) => {
                                if sid == service_id {
                                chosen_sid.push(service_id);
                                sp.pmt_pids[pid as usize] = 1;
                                sp.pids[pid as usize] = 1;
                                pid_pos.push(cnt);
                                sp.pmt_version.pid[sp.pmt_retain as usize] = pid;
                                sp.pmt_retain += 1;
                                debug!("pid={} , service_id={} \"{}\"", pid, service_id, sid);
                                };
                            },
                            Err(_err) => {
                            },
                        };
                    },
                };
            }

            // カウントアップ
            cnt += 4;

        }

        // available sidの編集
        let mut available_sid: String = "".to_string();
        for avail_sid in avail_sids {
            //available_sid = avail_sid.to_digit().unwrap();
            available_sid.push_str(&*format!("{} ",avail_sid as u16));
        };
        info!("Available sid = {}",available_sid);

        // chosen sidの編集
        let mut chosen_sids = "".to_string();
        for chosen in chosen_sid {
            chosen_sids.push_str(&*format!("{} ",chosen as u16));
        };
        info!("Chosen sid    = {}", chosen_sids);

        // available pmt編集
        let mut available_pmt: String = "".to_string();
        for avail_pmt in avail_pmts {
            available_pmt.push_str(&*format!("0x{:03x} ",avail_pmt));
        }
        info!("Available PMT = {}\r", available_pmt);

        // リターン情報
        result = recreate_pat(&mut sp, &data, &pid_pos);

    }

    result

}

// 新しいPATの作成
pub fn recreate_pat(sp: &mut Splitter, data: &[u8], pos: &Vec<usize>) -> i32 {

    let mut crc_data: Vec<u8> = vec![];


    // CRC32計算データの作成
    // チャンネルによって変わらない部分
    for cnt in 0..=(LENGTH_PAT_HEADER - 4) as usize {
        crc_data.push(data[cnt]);
    }

    // NIT
    crc_data.push(0x00);
    crc_data.push(0x00);
    crc_data.push(0xe0);
    crc_data.push(0x10);

    // チャンネルによって変わる部分
    for pos_num in pos {
        for cnt in 0..=3 {
            crc_data.push(data[pos_num + cnt]);
        }
    }

    // CRC32計算
    let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
    let checksum = crc.checksum(&crc_data[..]);

    // CRCを追加
    crc_data.push((checksum >> 24 & 0xff) as u8);
    crc_data.push((checksum >> 16 & 0xff) as u8);
    crc_data.push((checksum >>  8 & 0xff) as u8);
    crc_data.push((checksum       & 0xff) as u8);

    // 0xff埋め
    for _cnt in crc_data.len()..=LENGTH_PACKET - 1 {
        crc_data.push(0xff);
    }

    // PAT変数へ設定
    for cnt in 0..=LENGTH_PACKET - 1 {
        sp.pat[cnt] = crc_data[cnt];
    }

    // リターン情報
    TSS_SUCCESS

}

// PMT 解析処理
pub fn analyze_pmt(sp: &mut Splitter, data: &[u8], mark: u16) -> i32 {

    let mut n: i16;
    let mut retry_count = 0;
    let payload_offset: usize;

    // PIDの取得
    let pid = get_pid(&data[..]);

    // PES開始インジケータ
    if data[1] & 0x40 == 0x40  {

        // セクションサイズ取得(ヘッダ込)
        sp.section_remain[pid as usize] = (((data[6] as i16 & 0x0f) << 8) + data[7] as i16) as u16;
        payload_offset =  5;

        for cnt in 0..=sp.pmt_retain - 1 {
            if sp.pmt_version.pid[cnt as usize] == pid {
                sp.pmt_version.version[cnt as usize] = data[10] & 0x3e;
            }
        }

        // PCR取得
        let pcr = get_pid(&data[(payload_offset + 7)..]);
        sp.pids[pcr as usize] = mark;

        // ECM
        // ES情報開始点
        n = ((data[payload_offset + 10] as i16 & 0x0f) << 8) +
            (data[payload_offset + 11] as i16) + payload_offset as i16 + 12;
        let mut p = payload_offset as i16 + 12;

        while p < n {
            let tag: u32 = data[p as usize] as u32;
            let len: u32 = data[p as usize + 1] as u32;
            p += 2;

            if tag == 0x09 && len >= 4 && p as u32 + len <= n as u32 {
                let ca_pid = ((((data[p as usize + 2] as i16) << 8) | data[p as usize + 3] as i16) & 0x1fff) as usize;
                sp.pids[ca_pid] = mark;
            }

        }
    }
    else {
        // セクション先頭が飛んでいる場合
        if sp.section_remain[pid as usize] == 0 {
            return TSS_ERROR;
        }

        // パケットカウンタが飛んでいる場合
        if (data[3] & 0x0f) != ((sp.packet_seq[pid as usize] + 1) & 0x0f) {
            return TSS_ERROR;
        }
        payload_offset = 4;
        n = payload_offset as i16;

    }

    // 巡回カウンタ
    sp.packet_seq[pid as usize] = data[3] & 0x0f;


    let mut nall = sp.section_remain[pid as usize];
    if nall > (LENGTH_PACKET - payload_offset) as u16 {
        nall = (LENGTH_PACKET - payload_offset) as u16;
    }

    // ES PID
    while n <= (nall + payload_offset as u16 - 5) as i16 {

        // ストリーム種別が 0x0D（type D）は出力対象外
        if data[n as usize] != 0x0d {
            let epid = get_pid(&data[(n as usize)..]);
            sp.pids[epid as usize] = mark;
        };

        n += 4 + ((data[n as usize + 3] as i16 & 0x0F) << 8) as i16 + data[n as usize + 4] as i16 + 1;
        retry_count += 1;

        if retry_count > nall {
            debug!("TSS_ERROR");
            // リターン情報
            return TSS_ERROR;
        }
    }

    sp.section_remain[pid as usize] -= nall;

    if sp.section_remain[pid as usize] > 0 {
        // リターン情報
        return SECTION_CONTINUE;
    }
    else {
        // リターン情報
        return TSS_SUCCESS;
    }

}

// PID取得処理
pub fn get_pid(data: &[u8]) -> i16 {

    let pid: i16 = ((data[1] as i16 & 0x1f) << 8) + data[2] as i16;

    // リターン情報
    pid

}