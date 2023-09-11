use serde::Serialize;

pub const WASM_PAGE_SIZE: u64 = 65536;
// A block contains 64bits
pub const BLOCK_PER_PAGE_SIZE: u64 = WASM_PAGE_SIZE / 8;

const WASM_32_MAXIMAL_PAGES_DEFAULT: u32 = 65536;

#[derive(Serialize, Debug, Clone, Copy)]
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
