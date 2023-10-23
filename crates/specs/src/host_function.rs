use serde::Deserialize;
use serde::Serialize;

use crate::external_host_call_table::ExternalHostCallSignature;
use crate::types::ValueType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub params: Vec<ValueType>,
    pub return_type: Option<ValueType>,
}

#[derive(Debug)]
pub enum Error {
    DuplicateRegister,
}

#[derive(Debug, Clone)]
pub enum HostFunctionDesc {
    Internal {
        name: String,
        op_index_in_plugin: usize,
        plugin: HostPlugin,
    },
    External {
        name: String,
        op: usize,
        sig: ExternalHostCallSignature,
    },
}

impl HostFunctionDesc {
    pub fn name(&self) -> &String {
        match self {
            HostFunctionDesc::Internal { name, .. } | HostFunctionDesc::External { name, .. } => {
                name
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HostPlugin {
    HostInput = 0,
    Context,
    Require,
}
