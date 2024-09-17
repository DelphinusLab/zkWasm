use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::host_function::Signature;
use crate::types::ValueType;

pub mod encode;
mod table;

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalHostCallSignature {
    Argument,
    Return,
}

impl ExternalHostCallSignature {
    pub fn is_ret(&self) -> bool {
        *self == ExternalHostCallSignature::Return
    }
}

impl From<ExternalHostCallSignature> for Signature {
    fn from(sig: ExternalHostCallSignature) -> Signature {
        match sig {
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

#[derive(Serialize, Deserialize)]
pub struct ExternalHostCallEntry {
    pub op: usize,
    pub value: u64,
    pub is_ret: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ExternalHostCallTable(pub(crate) Vec<ExternalHostCallEntry>);

impl ExternalHostCallTable {
    pub fn new(entries: Vec<ExternalHostCallEntry>) -> Self {
        Self(entries)
    }

    pub fn entries(&self) -> &Vec<ExternalHostCallEntry> {
        &self.0
    }

    pub fn push(&mut self, entry: ExternalHostCallEntry) {
        self.0.push(entry);
    }

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let mut fd = std::fs::File::create(path)?;

        fd.write_all(serde_json::to_string_pretty(self).unwrap().as_bytes())?;

        Ok(())
    }

    pub fn read(path: &PathBuf) -> std::io::Result<Self> {
        let mut fd = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf)?;

        Ok(serde_json::from_slice(&buf).unwrap())
    }
}
