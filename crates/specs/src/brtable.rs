use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct BrTableEntry {
    pub fid: u32,
    pub iid: u32,
    pub index: u32,
    pub drop: u32,
    pub keep: u32,
    pub dst_pc: u32,
}

#[derive(Default, Serialize, Debug, Clone, Deserialize)]
pub struct BrTable(Vec<BrTableEntry>);

impl BrTable {
    pub fn new(entries: Vec<BrTableEntry>) -> Self {
        BrTable(entries)
    }

    pub fn entries(&self) -> &Vec<BrTableEntry> {
        &self.0
    }
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct ElemEntry {
    pub table_idx: u32,
    pub type_idx: u32,
    pub offset: u32,
    pub func_idx: u32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ElemTable(BTreeMap<(u32, u32), ElemEntry>);

#[derive(Serialize, Debug, Deserialize)]
struct Entry {
    key: (u32, u32),
    val: ElemEntry,
}

impl Serialize for ElemTable {
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

impl<'de> Deserialize<'de> for ElemTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Vec::<Entry>::deserialize(deserializer)
            .map(|mut v| ElemTable(v.drain(..).map(|kv| (kv.key, kv.val)).collect()))
    }
}

impl ElemTable {
    pub fn insert(&mut self, entry: ElemEntry) {
        self.0.insert((entry.table_idx, entry.offset), entry);
    }

    pub fn entries(&self) -> Vec<ElemEntry> {
        self.0.values().cloned().collect()
    }
}

pub enum IndirectClass {
    BrTable,
    CallIndirect,
}
