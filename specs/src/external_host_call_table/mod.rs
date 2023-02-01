use serde::Serialize;

use crate::{host_function::Signature, types::ValueType};

pub mod encode;
mod table;

#[derive(Copy, Clone, Debug, PartialEq, Serialize)]
pub enum ExternalHostCallSignature {
    Argument,
    Return,
}

impl ExternalHostCallSignature {
    pub fn is_ret(&self) -> bool {
        *self == ExternalHostCallSignature::Return
    }
}

impl Into<Signature> for ExternalHostCallSignature {
    fn into(self) -> Signature {
        match self {
            ExternalHostCallSignature::Argument => Signature {
                params: vec![ValueType::I64],
                return_type: None,
            },
            ExternalHostCallSignature::Return => Signature {
                params: vec![],
                return_type: Some(ValueType::I64),
            },
        }
    }
}

pub struct ExternalHostCallEntry {
    pub op: usize,
    pub value: u64,
    pub sig: ExternalHostCallSignature,
}

pub struct ExternalHostCallTable(Vec<ExternalHostCallEntry>);

impl ExternalHostCallTable {
    pub fn entries(&self) -> &Vec<ExternalHostCallEntry> {
        &self.0
    }
}
