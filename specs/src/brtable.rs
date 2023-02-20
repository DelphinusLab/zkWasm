use std::collections::BTreeMap;

use serde::Serialize;

use crate::utils::common_range::CommonRange;

#[derive(Serialize, Debug, Clone)]
pub struct BrTableEntry {
    pub fid: CommonRange,
    pub iid: CommonRange,
    pub index: CommonRange,
    pub drop: CommonRange,
    pub keep: CommonRange,
    pub dst_pc: CommonRange,
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
    pub table_idx: CommonRange,
    pub type_idx: CommonRange,
    pub offset: CommonRange,
    pub func_idx: CommonRange,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct ElemTable(BTreeMap<(CommonRange, CommonRange), ElemEntry>);

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
