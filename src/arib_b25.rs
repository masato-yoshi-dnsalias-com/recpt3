use std::fmt::Display;
use std::fmt::Formatter;
use std::io::{Error, ErrorKind};
use std::option::Option;
use std::os::raw::{c_int, c_void};
use std::result::Result;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct B_CAS_INIT_STATUS {
    pub system_key: [u8; 32usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct B_CAS_ID {
    pub data: *mut i64,
    pub count: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct B_CAS_PWR_ON_CTRL {
    pub s_yy: i32,
    pub s_mm: i32,
    pub s_dd: i32,
    pub l_yy: i32,
    pub l_mm: i32,
    pub l_dd: i32,
    pub hold_time: i32,
    pub broadcaster_group_id: i32,
    pub network_id: i32,
    pub transport_id: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct B_CAS_PWR_ON_CTRL_INFO {
    pub data: *mut B_CAS_PWR_ON_CTRL,
    pub count: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct B_CAS_ECM_RESULT {
    pub scramble_key: [u8; 16usize],
    pub return_code: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct B_CAS_CARD {
    pub private_data: *mut c_void,
    pub release: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void
        )
    >,
    pub init: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void
        ) -> c_int,
    >,
    pub get_init_status: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void,
            stat: *mut B_CAS_INIT_STATUS,
        ) -> c_int,
    >,
    pub get_id: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void,
            dst: *mut B_CAS_ID,
        ) -> c_int,
    >,
    pub get_pwr_on_ctrl: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void,
            dst: *mut B_CAS_PWR_ON_CTRL_INFO,
        ) -> c_int,
    >,
    pub proc_ecm: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void,
            dst: *mut B_CAS_ECM_RESULT,
            src: *mut u8,
            len: c_int,
        ) -> c_int,
    >,
    pub proc_emm: Option<
        unsafe extern "C" fn(
            bcas: *mut c_void,
            src: *mut u8,
            len: c_int,
        ) -> c_int,
    >,
    //pub(crate) _pinned: PhantomPinned,
}

//impl std::error::Error for AribB25DecoderError {}
impl std::error::Error for BCasCardError {}

#[derive(Debug, Clone)]
pub enum BCasCardError {
    BcasCardErrorInvalidParameter = -1,
    BcasCardErrorNotInitialized = -2,
    BcasCardErrorNoSmartCardReader = -3,
    BcasCardErrorAllReadersConnectionFailed = -4,
    BcasCardErrorNoEnoughMemory = -5,
    BcasCardErrorTransmitFailed = -6,
}

impl From<i32> for BCasCardError {
    fn from(e: i32) -> Self {
        if e == -1 {
            BCasCardError::BcasCardErrorInvalidParameter
        } else if e == -2 {
            BCasCardError::BcasCardErrorNotInitialized
        } else if e == -3 {
            BCasCardError::BcasCardErrorNoSmartCardReader
        } else if e == -4 {
            BCasCardError::BcasCardErrorAllReadersConnectionFailed
        } else if e == -5 {
            BCasCardError::BcasCardErrorNoEnoughMemory
        } else if e == -6 {
            BCasCardError::BcasCardErrorTransmitFailed
        } else {
            panic!("unknown error code: {}", e)
        }
    }
}

impl Display for BCasCardError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        type E = BCasCardError;
        match *self {
            E::BcasCardErrorInvalidParameter => {
                write!(f, "BcasCardErrorInvalidParameter")
            }
            E::BcasCardErrorNotInitialized => {
                write!(f, "BcasCardErrorNotInitialized")
            }
            E::BcasCardErrorNoSmartCardReader => {
                write!(f, "BcasCardErrorNoSmartCardReader")
            }
            E::BcasCardErrorAllReadersConnectionFailed => {
                write!(f, "BcasCardErrorAllReadersConnectionFailed")
            }
            E::BcasCardErrorNoEnoughMemory => {
                write!(f, "BcasCardErrorNoEnoughMemory")
            }
            E::BcasCardErrorTransmitFailed => {
                write!(f, "BcasCardErrorTransmitFailed")
            }
        }
    }
}

#[allow(dead_code)]
impl B_CAS_CARD {
    pub fn release(&mut self) {
        unsafe {
            if self.release.is_some() {
                self.release.unwrap()(
                    self as *mut _ as *mut c_void
                );
            }
        }
    }

    pub fn init(&self) -> i32 {
        unsafe {
            match self.init {
                Some(f) => f(
                    self as *const _ as *mut c_void
                    ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }

    pub fn get_init_status(&self, stat: &B_CAS_INIT_STATUS) -> i32 {
        unsafe {
            match self.get_init_status {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    stat as *const _ as *mut _,
                    ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }

    pub fn get_id(&self, dst: &B_CAS_ID) -> i32 {
        unsafe {
            match self.get_id {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    dst as *const _ as *mut _,
                    ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }

    pub fn get_pwr_on_ctrl(&self, dst: &B_CAS_ID) -> i32 {
        unsafe {
            match self.get_pwr_on_ctrl {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    dst as *const _ as *mut _,
                    ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }

    pub fn proc_ecm(&self, dst: &B_CAS_ID, src: *mut u8, len: i32) -> i32 {
        unsafe {
            match self.proc_ecm {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    dst as *const _ as *mut _,
                    src,
                    len,
                    ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }

    pub fn proc_emm(&self, src: *mut u8, len: i32) -> i32 {
        unsafe {
            match self.proc_emm {
                Some(f) => f(
                    self as *const _ as *mut c_void, src, len
                    ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }

    pub fn initialize(&mut self) -> Result<(), Error> {
        let init = self.init;
        let errno =
            unsafe { init.unwrap()(self as *mut B_CAS_CARD as *mut c_void) };

        if errno != 0 {
            Err(Error::new(ErrorKind::Other, BCasCardError::from(errno)))
        } else {
            Ok(())
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ARIB_STD_B25_BUFFER {
    pub data: *mut u8,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ARIB_STD_B25_PROGRAM_INFO {
    pub program_number: i32,
    pub ecm_unpurchased_count: i32,
    pub last_ecm_error_code: i32,
    pub padding: i32,
    pub total_packet_count: i64,
    pub undecrypted_packet_count: i64,
}


#[repr(C)]
#[derive(Debug, Clone)]
pub struct ARIB_STD_B25 {
    pub private_data: *mut c_void,
    pub release: Option<unsafe extern "C" fn(
        std_b25: *mut c_void)
    >,
    pub set_multi2_round: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            round: i32,
        ) -> c_int,
    >,
    pub set_strip: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            strip: i32,
        ) -> c_int,
    >,
    pub set_emm_proc: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            on: i32,
        ) -> c_int,
    >,
    // tsukumijima版のlibarib25の場合に有効にする
    //pub set_simd_mode: Option<
    //    unsafe extern "C" fn(
    //        std_b25: *mut c_void,
    //        instructin: i32,
    //    ) -> c_int,
    //>,
    // tsukumijima版のlibarib25の場合に有効にする
    //pub get_simd_mode:
    //    Option<unsafe extern "C" fn(std_b25: *mut c_void) -> i32>,
    pub set_b_cas_card: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            bcas: *mut B_CAS_CARD,
        ) -> c_int,
    >,
    // tsukumijima版のlibarib25の場合に有効にする
    //pub set_unit_size: Option<
    //    unsafe extern "C" fn(
    //        std_b25: *mut c_void,
    //        size: c_int,
    //    ) -> c_int,
    //>,
    pub reset: Option<
        unsafe extern "C" fn(std_b25: *mut c_void) -> c_int,
    >,
    pub flush: Option<
        unsafe extern "C" fn(std_b25: *mut c_void) -> c_int,
    >,
    pub put: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            buf: *mut ARIB_STD_B25_BUFFER,
        ) -> c_int,
    >,
    pub get: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            buf: *mut ARIB_STD_B25_BUFFER,
        ) -> c_int,
    >,
    pub get_program_count: Option<
        unsafe extern "C" fn(std_b25: *mut c_void) -> c_int,
    >,
    pub get_program_info: Option<
        unsafe extern "C" fn(
            std_b25: *mut c_void,
            info: *mut ARIB_STD_B25_PROGRAM_INFO,
            idx: i32,
        ) -> c_int,
    >,
    // tsukumijima版のlibarib25の場合に有効にする
    //pub withdraw: Option<
    //    unsafe extern "C" fn(
    //        std_b25: *mut c_void,
    //        buf: *mut ARIB_STD_B25_BUFFER,
    //    ) -> c_int,
    //>,
}

#[allow(dead_code)]
impl ARIB_STD_B25 {
    pub fn release(&mut self) {
        unsafe {
            if self.release.is_some() {
                self.release.unwrap()(self as *mut _ as *mut c_void);
            }
        }
    }
    pub fn set_multi2_round(&self, round: i32) -> c_int {
        unsafe {
            match self.set_multi2_round {
                Some(f) => f(
                    self as *const _ as *mut c_void, round
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn set_strip(&self, strip: i32) -> c_int {
        unsafe {
            match self.set_strip {
                Some(f) => f(
                    self as *const _ as *mut c_void, strip
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn set_emm_proc(&self, on: i32) -> c_int {
        unsafe {
            match self.set_emm_proc {
                Some(f) => f(
                    self as *const _ as *mut c_void, on
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    // tsukumijima版のlibarib25の場合に有効にする
    //pub fn set_simd_mode(&self, instruction_type: i32) -> c_int {
    //    unsafe {
    //        match self.set_simd_mode {
    //            Some(f) => f(
    //                self as *const _ as *mut c_void,
    //                instruction_type,
    //            ),
    //            None => unreachable!("Maybe uninitialized"),
    //        }
    //    }
    //}
    //pub fn get_simd_mode(&self) -> i32 {
    //    unsafe {
    //        match self.get_simd_mode {
    //            Some(f) => f(
    //                self as *const _ as *mut c_void
    //            ),
    //            None => unreachable!("Maybe uninitialized"),
    //        }
    //    }
    //}
    pub fn set_b_cas_card(&self, bcas: &B_CAS_CARD) -> c_int {
        unsafe {
            match self.set_b_cas_card {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    bcas as *const _ as *mut _,
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    // tsukumijima版のlibarib25の場合に有効にする
    //pub fn set_unit_size(&self, size: c_int) -> c_int {
    //    unsafe {
    //        match self.set_unit_size {
    //            Some(f) => f(
    //                self as *const _ as *mut c_void, size
    //            ),
    //            None => unreachable!("Maybe uninitialized"),
    //        }
    //    }
    //}
    pub fn reset(&self) -> c_int {
        unsafe {
            match self.reset {
                Some(f) => f(
                    self as *const _ as *mut c_void
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn flush(&self) -> c_int {
        unsafe {
            match self.flush {
                Some(f) => f(
                    self as *const _ as *mut c_void
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn put(&self, buf: &ARIB_STD_B25_BUFFER) -> c_int {
        unsafe {
            match self.put {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    buf as *const _ as *mut _,
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn get(&self, buf: &mut ARIB_STD_B25_BUFFER) -> c_int {
        unsafe {
            match self.get {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    buf as *mut _,
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn get_program_count(&self) -> i32 {
        unsafe {
            match self.get_program_count {
                Some(f) => f(
                    self as *const _ as *mut c_void
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    pub fn get_program_info(&self, info: &mut ARIB_STD_B25_PROGRAM_INFO, idx: i32) -> i32 {
        unsafe {
            match self.get_program_info {
                Some(f) => f(
                    self as *const _ as *mut c_void,
                    info as *mut _,
                    idx,
                ),
                None => unreachable!("Maybe uninitialized"),
            }
        }
    }
    // tsukumijima版のlibarib25の場合に有効にする
    //pub fn withdraw(&self, buf: &mut ARIB_STD_B25_BUFFER) -> i32 {
    //    unsafe {
    //        match self.withdraw {
    //            Some(f) => f(
    //                self as *const _ as *mut c_void,
    //                buf as *mut _,
    //            ),
    //            None => unreachable!("Maybe uninitialized"),
    //        }
    //    }
    //}
}
