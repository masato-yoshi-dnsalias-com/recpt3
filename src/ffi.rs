
use crate::arib_b25::{ARIB_STD_B25, B_CAS_CARD};

extern "C" {
    pub fn create_arib_std_b25() -> *mut ARIB_STD_B25;
}

extern "C" {
    pub fn create_b_cas_card() -> *mut B_CAS_CARD;
}
