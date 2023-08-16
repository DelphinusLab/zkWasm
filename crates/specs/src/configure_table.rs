use serde::{Serialize, Deserialize};

pub const WASM_PAGE_SIZE: u64 = 65536;

const WASM_32_MAXIMAL_PAGES_DEFAULT: u32 = 65536;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ConfigureTable {
    pub init_memory_pages: u32,
    pub maximal_memory_pages: u32,
}

impl Default for ConfigureTable {
    fn default() -> Self {
        Self {
            init_memory_pages: 0,
            maximal_memory_pages: WASM_32_MAXIMAL_PAGES_DEFAULT,
        }
    }
}
