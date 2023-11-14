use std::env;
use std::sync::Mutex;

pub const POW_TABLE_POWER_START: u64 = 128;

pub const MIN_K: u32 = 18;

lazy_static! {
    static ref ZKWASM_K: Mutex<u32> =
        Mutex::new(env::var("ZKWASM_K").map_or(MIN_K, |k| k.parse().unwrap()));
}

pub fn set_zkwasm_k(k: u32) {
    assert!(k >= MIN_K);

    let mut zkwasm_k = (*ZKWASM_K).lock().unwrap();
    *zkwasm_k = k;
}

pub fn zkwasm_k() -> u32 {
    *ZKWASM_K.lock().unwrap()
}

pub fn init_zkwasm_runtime(k: u32) {
    set_zkwasm_k(k);
}
