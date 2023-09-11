use specs::configure_table::WASM_BYTES_PER_PAGE;

const WASM_BLOCK_BYTE_SIZE_SHIFT: u32 = 3;

/// Get offset within a block
pub(crate) const WASM_BLOCK_BYTE_OFFSET_MASK: u32 = 0b111;
/// The block number of a WASM page
pub(crate) const WASM_BLOCKS_PER_PAGE: u32 = WASM_BYTES_PER_PAGE as u32 / u8::BITS;
/// A block has 8 bytes
pub(crate) const WASM_BLOCK_BYTE_SIZE: u32 = 1 << WASM_BLOCK_BYTE_SIZE_SHIFT;

pub(crate) fn block_from_address(address: u32) -> u32 {
    address >> WASM_BLOCK_BYTE_SIZE_SHIFT
}

pub(crate) fn byte_offset_from_address(address: u32) -> u32 {
    address & WASM_BLOCK_BYTE_OFFSET_MASK
}
