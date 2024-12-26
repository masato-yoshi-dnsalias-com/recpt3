#[allow(dead_code)]
pub const PROGRAM_RECPT: &str = "recpt3";
#[allow(dead_code)]
pub const PROGRAM_CHECKSIGNAL: &str = "checksignal";
#[allow(dead_code)]
pub const PROGRAM_TS_SPLITTER: &str = "ts_splitter";
#[allow(dead_code)]
pub const PROGRAM_DROP_CHECK: &str = "drop_check";
#[allow(dead_code)]
pub const TRUE: i32 = 1;
#[allow(dead_code)]
pub const FALSE: i32 = 0;

// Struct DecoderOptions
#[derive(Debug, Copy, Clone)]
pub struct DecoderOptions {
    pub round: i32,
    pub strip: i32,
    pub emm: i32,
}

// struct CommanLineOpt
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommanLineOpt {
    pub _program: String,
    //pub use_bell: bool,
    pub use_b25: bool,
    //pub use_udp: bool,
    pub _use_http: bool,
    pub _http_port: u16,
    //pub host_to: String,
    //pub port_to: u16,
    pub device: String,
    pub sid_list: String,
    pub use_splitter: bool,
    pub _use_round: bool,
    pub _use_lnb: bool,
    pub _lnb: u64,
    pub _use_device: bool,
    pub channel: String,
    pub duration: u64,
    pub infile: String,
    pub outfile: String,
}
