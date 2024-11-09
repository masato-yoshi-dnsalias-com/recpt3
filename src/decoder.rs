use log::{error};
use std::ptr::null_mut;

use crate::arib_b25::{ARIB_STD_B25, ARIB_STD_B25_BUFFER,  B_CAS_CARD};
use crate::ffi::{create_arib_std_b25, create_b_cas_card};

// B25デコードの初期設定
#[allow(dead_code)]
pub unsafe fn b25_startup(round: i32, strip: i32, emm: i32 ) -> (i32, *mut ARIB_STD_B25, *mut B_CAS_CARD) {

    let dec = create_arib_std_b25();
    let mut result: i32;

    // Multi2暗号化の設定
    result = dec.as_ref().expect("set_multi2_round failed").set_multi2_round(round);
    if result < 0 {
        error!("set_multi2_round failed(result={})", result);
    };

    // Strip設定
    result = dec.as_ref().expect("set_strip failed").set_strip(strip);
    if result < 0 {
        error!("set_strip failedi(result={})", result);
    };

    // EMM設定
    result = dec.as_ref().expect("set_emm_proc failed").set_emm_proc(emm);
    if result < 0 {
        error!("set_emm_proc failed((result={})", result)
    };

    // ICカード設定
    let bcas = create_b_cas_card();

    // カードリーダー初期化
    result = bcas.as_ref().expect("init failed").init();
    if result < 0 {
        error!("init failed(result={})", result);
    };

    // BCASカード設定
    result = dec.as_ref().expect("set_b_cas_card failed").set_b_cas_card(bcas.as_ref().unwrap());
    if result < 0 {
        error!("set_b_cas_card failed(result={})", result);
    };

    (result, dec, bcas)

}

// B25デコード処理
//pub unsafe fn b25_decode<'a, 'b>(dec: *mut ARIB_STD_B25, sbuf: &'a ARIB_STD_B25_BUFFER) -> &'b [u8] {
#[allow(dead_code)]
pub unsafe fn b25_decode(dec: *mut ARIB_STD_B25, sbuf: &ARIB_STD_B25_BUFFER) -> (&[u8], i32) {

    let mut result;

    // BCASカードへデータ送信
    //debug!("put len = {}", sbuf.size);
    result = dec.as_ref().expect("b25->put failed").put(sbuf);
    if result < 0 { error!("b25->put failed") };

    // BCASカードからの受信バッファ
    let mut buffer_struct = ARIB_STD_B25_BUFFER {
        data: null_mut(),
        size: 0,
    };

    // BCASカードからデータ受信
    result = dec.as_ref().expect("b25->get failed").get(&mut buffer_struct);
    if result < 0 { error!("b25->get failed(result={})", result) };
    //debug!("rc={} , get len = {}", result,buffer_struct.size);

    match buffer_struct.size {
        0 => (&[0], 0),
        _ => {
            let buff = core::slice::from_raw_parts_mut(buffer_struct.data, buffer_struct.size as usize);
            (buff, buffer_struct.size.try_into().unwrap())
        },
    }
}

// B25シャットダウン処理
#[allow(dead_code)]
pub unsafe fn b25_shutdown(dec: *mut ARIB_STD_B25, bcas: *mut B_CAS_CARD) -> () {

    dec.as_mut().expect("b25->release failed").release();
    bcas.as_mut().unwrap().release();

}
