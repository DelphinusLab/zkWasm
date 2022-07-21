use serde::Serialize;

use crate::mtable::VarType;

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

impl Into<VarType> for Value {
    fn into(self) -> VarType {
        match self {
            Value::I32(_) => VarType::I32,
            Value::I64(_) => VarType::I64,
            Value::U32(_) => VarType::U32,
            Value::U64(_) => VarType::U64,
        }
    }
}

impl Value {
    pub fn internal(&self) -> u64 {
        match self {
            Value::I32(v) => (*v) as u64,
            Value::I64(v) => (*v) as u64,
            Value::U32(v) => (*v) as u64,
            Value::U64(v) => (*v) as u64,
        }
    }
}

#[derive(Debug)]
pub enum CompileError {}

#[derive(Debug)]
pub enum ExecutionError {}
