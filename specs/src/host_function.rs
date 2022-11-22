use serde::Serialize;

use crate::types::ValueType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Signature {
    pub params: Vec<ValueType>,
    pub return_type: Option<ValueType>,
}

#[derive(Debug)]
pub enum Error {
    DuplicateRegisterFunction,
    DuplicateRegisterGlobal,
    DuplicateRegisterMemory,
}

#[derive(Debug, Clone)]
pub struct HostFunctionDesc {
    pub name: String,
    pub op_index_in_plugin: usize,
    pub plugin: HostPlugin,
}

#[derive(Clone, Debug, Serialize, Copy, PartialEq)]
pub enum HostPlugin {
    HostInput = 0,
    Sha256,
}
