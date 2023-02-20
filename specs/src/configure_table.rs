use serde::Serialize;

use crate::utils::common_range::CommonRange;

pub const WASM_PAGE_SIZE: u32 = 65536;

const WASM_32_MAXIMAL_PAGES_DEFAULT: u32 = 65536;

#[derive(Serialize, Debug, Clone, Copy)]
pub struct ConfigureTable {
    pub init_memory_pages: CommonRange,
    pub maximal_memory_pages: u32,
}

impl Default for ConfigureTable {
    fn default() -> Self {
        Self {
            init_memory_pages: CommonRange::from(0u32),
            maximal_memory_pages: WASM_32_MAXIMAL_PAGES_DEFAULT,
        }
    }
}
