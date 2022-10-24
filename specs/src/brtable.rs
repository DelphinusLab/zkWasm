use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct BrTableEntry {
    pub moid: u16,
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
