use std::collections::HashMap;

use crate::mtable::LocationType;
use crate::mtable::VarType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub offset: u32,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
    pub eid: u32,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct InitMemoryTable(pub HashMap<(LocationType, u32), InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        let mut map = HashMap::new();

        entries.into_iter().for_each(|entry| {
            map.insert((entry.ltype, entry.offset), entry);
        });

        Self(map)
    }

    pub fn try_find(&self, ltype: LocationType, offset: u32) -> Option<&InitMemoryTableEntry> {
        self.0.get(&(ltype, offset))
    }
}
