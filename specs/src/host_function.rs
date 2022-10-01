use serde::Serialize;

use crate::types::ValueType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Signature {
    pub params: Vec<ValueType>,
    pub return_type: Option<ValueType>,
}

#[derive(Debug)]
pub enum Error {
    DuplicateRegister,
}

pub enum HostFunction {
    WasmInput = 0,
}
