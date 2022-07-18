use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ValueType {
    I32,
    I64,
    U32,
    U64,
}

impl From<parity_wasm::elements::ValueType> for ValueType {
    fn from(v: parity_wasm::elements::ValueType) -> Self {
        match v {
            parity_wasm::elements::ValueType::I32 => ValueType::I32,
            parity_wasm::elements::ValueType::I64 => ValueType::I64,
            parity_wasm::elements::ValueType::F32 => todo!(),
            parity_wasm::elements::ValueType::F64 => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
}

#[derive(Debug)]
pub enum CompileError {}

#[derive(Debug)]
pub enum ExecutionError {}
