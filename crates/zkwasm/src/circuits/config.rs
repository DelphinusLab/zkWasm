use std::sync::Mutex;

pub const POW_TABLE_POWER_START: u64 = 128;

pub const MIN_K: u32 = 18;
const MAX_K: u32 = 22;

lazy_static! {
    static ref ZKWASM_K: Mutex<Option<u32>> = Mutex::new(None);
}

pub(crate) fn set_zkwasm_k(k: u32) {
    assert!(k >= MIN_K);
    assert!(k <= MAX_K);

    let mut zkwasm_k = (*ZKWASM_K).lock().unwrap();
    *zkwasm_k = Some(k);
}

pub(in crate::circuits) fn zkwasm_k() -> u32 {
    ZKWASM_K
        .lock()
        .unwrap()
        .expect("ZKWASM_K is not set, please make sure 'init_zkwasm_runtime' have called.")
}

pub(crate) fn init_zkwasm_runtime(k: u32) {
    set_zkwasm_k(k);
}

pub(crate) fn common_range(k: u32) -> u32 {
    (1 << k) - 256
}

pub(crate) fn common_range_max(k: u32) -> u32 {
    common_range(k) - 1
}
