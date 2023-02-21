use crate::mtable::{LocationType, VarType};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub offset: u32,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct InitMemoryTable(Vec<InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        let mut imtable = Self(entries);
        imtable.sort();
        imtable
    }

    pub fn entries(&self) -> &Vec<InitMemoryTableEntry> {
        &self.0
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }

    pub fn try_find(&self, ltype: LocationType, offset: u32) -> Option<u64> {
        for entry in self.0.iter() {
            if entry.ltype == ltype && entry.offset == offset {
                return Some(entry.value);
            }
        }

        None
    }

    fn sort(&mut self) {
        self.0.sort_by_key(|item| (item.ltype, item.offset))
    }

    pub fn filter(&self, ltype: LocationType) -> Vec<&InitMemoryTableEntry> {
        self.0.iter().filter(|e| e.ltype == ltype).collect()
    }

    pub fn first_consecutive_zero_memory(&self) -> u32 {
        self.0.last().map_or(0, |entry| entry.offset + 1)
    }
}
