use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrTableEntry {
    pub fid: u32,
    pub iid: u32,
    pub index: u32,
    pub drop: u32,
    pub keep: u32,
    pub dst_pc: u32,
}

#[derive(Debug)]
pub struct BrTable(Vec<BrTableEntry>);

impl BrTable {
    pub fn new(entries: Vec<BrTableEntry>) -> Self {
        BrTable(entries)
    }

    pub fn entries(&self) -> &Vec<BrTableEntry> {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElemEntry {
    pub table_idx: u32,
    pub type_idx: u32,
    pub offset: u32,
    pub func_idx: u32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ElemTable(BTreeMap<(u32, u32), ElemEntry>);

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
