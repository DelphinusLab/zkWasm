pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;
pub mod test;

pub const RESERVED_INSTANCES_NUMBER: usize = if cfg!(feature = "checksum") { 1 } else { 0 };

enum Op {
    WasmInput = 0,
    WasmOutput = 1,
}
