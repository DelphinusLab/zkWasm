use serde::Serialize;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum LocationType {
    Heap = 0,
    Stack = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum AccessType {
    Read = 1,
    Write = 2,
    Init = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize)]
pub enum VarType {
    U8 = 1,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
}

impl From<parity_wasm::elements::ValueType> for VarType {
    fn from(v: parity_wasm::elements::ValueType) -> Self {
        match v {
            parity_wasm::elements::ValueType::I32 => Self::I32,
            parity_wasm::elements::ValueType::I64 => Self::I64,
            _ => todo!(),
        }
    }
}

impl From<crate::types::ValueType> for VarType {
    fn from(v: crate::types::ValueType) -> Self {
        match v {
            crate::types::ValueType::I32 => Self::I32,
            crate::types::ValueType::I64 => Self::I64,
            _ => todo!(),
        }
    }
}

impl VarType {
    pub fn byte_size(&self) -> u64 {
        match self {
            VarType::U8 => 1,
            VarType::I8 => 1,
            VarType::U16 => 2,
            VarType::I16 => 2,
            VarType::U32 => 4,
            VarType::I32 => 4,
            VarType::U64 => 8,
            VarType::I64 => 8,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MemoryTableEntry {
    pub eid: u64,
    // emid is small memory id of eid,
    // e.g. an opcode get a value front stack top and change it,
    // its event has two memory ops on the same memory address,
    // we should have use emid to seq the r/w op, it is an incremental value starting from 1
    pub emid: u64,
    pub mmid: u64,
    pub offset: u64,
    pub ltype: LocationType,
    pub atype: AccessType,
    pub vtype: VarType,
    pub value: u64,
}

impl MemoryTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn is_same_location(&self, other: &MemoryTableEntry) -> bool {
        self.mmid == other.mmid && self.offset == other.offset && self.ltype == other.ltype
    }
}

#[derive(Default, Debug, Serialize)]
pub struct MTable(Vec<MemoryTableEntry>);

impl MTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn new(entries: Vec<MemoryTableEntry>) -> Self {
        let mut mtable = MTable(entries);
        mtable.sort();
        mtable
    }

    fn sort(&mut self) {
        self.0
            .sort_by_key(|item| (item.ltype, item.mmid, item.eid, item.emid))
    }

    pub fn entries(&self) -> &Vec<MemoryTableEntry> {
        &self.0
    }
}
