use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct BrTableEntry {
    pub fid: u16,
    pub iid: u16,
    pub index: u16,
    pub drop: u16,
    pub keep: u16,
    pub dst_pc: u16,
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

#[derive(Serialize, Debug, Clone)]
pub struct ElemEntry {
    pub table_idx: u32,
    pub type_idx: u32,
    pub offset: u32,
    pub func_idx: u32,
}

#[derive(Debug, Default, Serialize, Clone)]
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
