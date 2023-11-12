pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;
pub mod test;

enum Op {
    WasmInput = 0,
    WasmOutput = 1,
    WasmWitnessInject = 2,
    WasmWitnessPop = 3,
}
