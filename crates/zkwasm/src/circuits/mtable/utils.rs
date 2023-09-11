use super::WASM_BLOCK_BYTE_OFFSET_MASK;
use super::WASM_BLOCK_BYTE_SIZE_SHIFT;

pub(crate) fn block_from_address(address: u32) -> u32 {
    address >> WASM_BLOCK_BYTE_SIZE_SHIFT
}

pub(crate) fn byte_offset_from_address(address: u32) -> u32 {
    address & WASM_BLOCK_BYTE_OFFSET_MASK
}
