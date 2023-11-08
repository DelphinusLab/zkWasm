use serde::Deserialize;
use serde::Serialize;

use crate::external_host_call_table::ExternalHostCallSignature;
use crate::host_function::HostPlugin;
use crate::mtable::VarType;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueType {
    I32,
    I64,
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
}

impl Into<VarType> for Value {
    fn into(self) -> VarType {
        match self {
            Value::I32(_) => VarType::I32,
            Value::I64(_) => VarType::I64,
        }
    }
}

impl Value {
    pub fn internal(&self) -> u64 {
        match self {
            Value::I32(v) => (*v) as u64,
            Value::I64(v) => (*v) as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum FunctionType {
    WasmFunction,
    HostFunction {
        plugin: HostPlugin,
        function_index: usize,
        function_name: String,
        op_index_in_plugin: usize,
    },
    HostFunctionExternal {
        function_name: String,
        op: usize,
        sig: ExternalHostCallSignature,
    },
}
