use crate::mtable::{LocationType, VarType};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub mmid: u64,
    pub offset: u64,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct InitMemoryTable(pub Vec<InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        let mut imtable = Self(entries);
        imtable.sort();
        imtable
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

    fn sort(&mut self) {
        self.0
            .sort_by_key(|item| (item.ltype, item.mmid, item.offset))
    }

    pub fn filter(&self, ltype: LocationType) -> Vec<&InitMemoryTableEntry> {
        self.0.iter().filter(|e| e.ltype == ltype).collect()
    }
}
