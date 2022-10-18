use serde::Serialize;
use crate::mtable::LocationType;

#[derive(Serialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub mmid: u64,
    pub offset: u64,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct InitMemoryTable(pub Vec<InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        Self(entries)
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }

    pub fn find(&self, ltype: LocationType, mmid: u64, offset: u64) -> u64 {
        for entry in self.0.iter() {
            if entry.ltype == ltype && entry.mmid == mmid && entry.offset == offset {
                return entry.value;
            }
        }

        unreachable!()
    }
}
