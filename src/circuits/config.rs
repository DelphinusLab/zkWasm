use std::{env, sync::Mutex};

pub const POW_TABLE_LIMIT: u64 = 128;

pub const MIN_K: u32 = 18;

lazy_static! {
    static ref ZKWASM_K: Mutex<u32> =
        Mutex::new(env::var("ZKWASM_K").map_or(MIN_K, |k| k.parse().unwrap()));
    pub(super) static ref ZKWASM_TABLE_DENOMINATOR: u32 =
        env::var("ZKWASM_TABLE_DENOMINATOR").map_or(32, |k| k.parse().unwrap());
    static ref ZKWASM_ETABLE_RATIO: u32 =
        env::var("ZKWASM_ETABLE_RATIO").map_or(31, |k| k.parse().unwrap());
    static ref ZKWASM_MTABLE_RATIO: u32 =
        env::var("ZKWASM_MTABLE_RATIO").map_or(31, |k| k.parse().unwrap());
    static ref ZKWASM_JTABLE_RATIO: u32 =
        env::var("ZKWASM_JTABLE_RATIO").map_or(31, |k| k.parse().unwrap());
    pub(super) static ref ZKWASM_FOREIGN_CALL_TABLE_RATIO: u32 =
        env::var("ZKWASM_FOREIGN_CALL_TABLE_RATIO").map_or(31, |k| k.parse().unwrap());
    static ref ZKWASM_SHA256_RATIO: u32 =
        env::var("ZKWASM_SHA256_RATIO").map_or(31, |k| k.parse().unwrap());
}

pub fn set_zkwasm_k(k: u32) {
    assert!(k >= MIN_K);

    let mut zkwasm_k = (*ZKWASM_K).lock().unwrap();
    *zkwasm_k = k;
}

pub fn zkwasm_k() -> u32 {
    *ZKWASM_K.lock().unwrap()
}

pub(crate) fn max_etable_rows() -> u32 {
    assert!(*ZKWASM_ETABLE_RATIO < *ZKWASM_TABLE_DENOMINATOR);

    (1 << zkwasm_k()) / *ZKWASM_TABLE_DENOMINATOR * *ZKWASM_ETABLE_RATIO
}

pub(crate) fn max_mtable_rows() -> u32 {
    assert!(*ZKWASM_MTABLE_RATIO < *ZKWASM_TABLE_DENOMINATOR);

    (1 << zkwasm_k()) / *ZKWASM_TABLE_DENOMINATOR * *ZKWASM_MTABLE_RATIO
}

pub(crate) fn max_jtable_rows() -> u32 {
    assert!(*ZKWASM_JTABLE_RATIO < *ZKWASM_TABLE_DENOMINATOR);

    (1 << zkwasm_k()) / *ZKWASM_TABLE_DENOMINATOR * *ZKWASM_JTABLE_RATIO
}

pub(crate) fn max_sha256_rows() -> u32 {
    assert!(*ZKWASM_SHA256_RATIO < *ZKWASM_TABLE_DENOMINATOR);

    (1 << zkwasm_k()) / *ZKWASM_TABLE_DENOMINATOR * *ZKWASM_SHA256_RATIO
}
