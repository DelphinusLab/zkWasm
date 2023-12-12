use std::collections::BTreeMap;

use crate::mtable::LocationType;
use crate::mtable::VarType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub offset: u32,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
    pub eid: u32,
}

#[derive(Default, Debug, Clone)]
pub struct InitMemoryTable(pub BTreeMap<(LocationType, u32), InitMemoryTableEntry>);

#[derive(Serialize, Debug, Deserialize)]
struct Entry {
    key: (LocationType, u32),
    val: InitMemoryTableEntry,
}

impl Serialize for InitMemoryTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.0.iter().map(|(key, val)| Entry {
            key: key.clone(),
            val: val.clone(),
        }))
    }
}

impl<'de> Deserialize<'de> for InitMemoryTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Vec::<Entry>::deserialize(deserializer)
            .map(|mut v| InitMemoryTable(v.drain(..).map(|kv| (kv.key, kv.val)).collect()))
    }
}

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        let mut map = BTreeMap::new();

        entries.into_iter().for_each(|entry| {
            map.insert((entry.ltype, entry.offset), entry);
        });

        Self(map)
    }

    pub fn entries(&self) -> &BTreeMap<(LocationType, u32), InitMemoryTableEntry> {
        &self.0
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }

    pub fn try_find(&self, ltype: LocationType, offset: u32) -> Option<&InitMemoryTableEntry> {
        self.0.get(&(ltype, offset))
    }
}
