use std::collections::HashSet;
use std::env;
use std::sync::Mutex;

use specs::itable::OpcodeClassPlain;
use specs::CompilationTable;

pub const POW_TABLE_LIMIT: u64 = 128;

pub const MIN_K: u32 = 18;

lazy_static! {
    static ref ZKWASM_K: Mutex<u32> =
        Mutex::new(env::var("ZKWASM_K").map_or(MIN_K, |k| k.parse().unwrap()));
}

#[derive(Clone)]
pub struct CircuitConfigure {
    pub initial_memory_pages: u32,
    pub maximal_memory_pages: u32,
    pub opcode_selector: HashSet<OpcodeClassPlain>,
}

#[thread_local]
static mut CIRCUIT_CONFIGURE: Option<CircuitConfigure> = None;

impl CircuitConfigure {
    #[allow(non_snake_case)]
    pub(crate) fn set_global_CIRCUIT_CONFIGURE(self) {
        unsafe {
            CIRCUIT_CONFIGURE = Some(self);
        }
    }

    pub(crate) fn get() -> CircuitConfigure {
        unsafe {
            if CIRCUIT_CONFIGURE.is_none() {
                panic!("CIRCUIT_CONFIGURE is not set, call init_zkwasm_runtime before configuring circuit.");
            } else {
                return CIRCUIT_CONFIGURE.clone().unwrap();
            }
        }
    }
}

impl From<&CompilationTable> for CircuitConfigure {
    fn from(table: &CompilationTable) -> Self {
        CircuitConfigure {
            initial_memory_pages: table.configure_table.init_memory_pages,
            maximal_memory_pages: table.configure_table.maximal_memory_pages,
            opcode_selector: table.itable.opcode_class(),
        }
    }
}

pub fn set_zkwasm_k(k: u32) {
    assert!(k >= MIN_K);

    let mut zkwasm_k = (*ZKWASM_K).lock().unwrap();
    *zkwasm_k = k;
}

pub fn zkwasm_k() -> u32 {
    *ZKWASM_K.lock().unwrap()
}

pub fn init_zkwasm_runtime(k: u32, table: &CompilationTable) {
    set_zkwasm_k(k);

    CircuitConfigure::from(table).set_global_CIRCUIT_CONFIGURE();
}

#[cfg(feature = "checksum")]
pub(crate) fn max_image_table_rows() -> u32 {
    8192
}
