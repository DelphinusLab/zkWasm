use serde::Serialize;

use crate::types::ValueType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Signature {
    pub params: Vec<ValueType>,
    pub return_type: Option<ValueType>,
}

pub const TIME_FUNC_INDEX: usize = 0;
