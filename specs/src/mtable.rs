use std::collections::HashSet;

use serde::Serialize;
use strum_macros::EnumIter;

use crate::imtable::InitMemoryTable;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
pub enum LocationType {
    Stack = 1,
    Heap = 2,
    Global = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Hash, Eq)]
pub enum InitType {
    Positive,
    Lazy,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Hash, Eq)]
pub enum AccessType {
    Read,
    Write,
    Init(InitType),
}

impl AccessType {
    pub fn into_index(&self) -> u64 {
        match self {
            AccessType::Read => Self::read_index(),
            AccessType::Write => Self::write_index(),
            AccessType::Init(_) => Self::init_index(),
        }
    }

    pub fn read_index() -> u64 {
        1
    }
    pub fn write_index() -> u64 {
        2
    }
    pub fn init_index() -> u64 {
        3
    }

    pub fn is_init(&self) -> bool {
        if let AccessType::Init(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_positive_init(&self) -> bool {
        if let AccessType::Init(InitType::Positive) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Hash, Eq)]
pub enum VarType {
    I32 = 1,
    I64,
}

impl VarType {
    pub fn byte_size(&self) -> u64 {
        match self {
            VarType::I32 => 4,
            VarType::I64 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Hash, Eq)]
pub enum MemoryReadSize {
    U8 = 1,
    S8,
    U16,
    S16,
    U32,
    S32,
    I64,
}

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Hash, Eq)]
pub enum MemoryStoreSize {
    Byte8 = 1,
    Byte16,
    Byte32,
    Byte64,
}

impl MemoryStoreSize {
    pub fn byte_size(&self) -> u64 {
        match self {
            MemoryStoreSize::Byte8 => 1,
            MemoryStoreSize::Byte16 => 2,
            MemoryStoreSize::Byte32 => 4,
            MemoryStoreSize::Byte64 => 8,
        }
    }
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
        }
    }
}

impl MemoryReadSize {
    pub fn byte_size(&self) -> u64 {
        match self {
            MemoryReadSize::U8 => 1,
            MemoryReadSize::S8 => 1,
            MemoryReadSize::U16 => 2,
            MemoryReadSize::S16 => 2,
            MemoryReadSize::U32 => 4,
            MemoryReadSize::S32 => 4,
            MemoryReadSize::I64 => 8,
        }
    }

    pub fn is_sign(&self) -> bool {
        match self {
            MemoryReadSize::U8
            | MemoryReadSize::U16
            | MemoryReadSize::U32
            | MemoryReadSize::I64 => false,
            _ => true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Hash, Eq, PartialEq)]
pub struct MemoryTableEntry {
    pub eid: u32,
    /*
       Emid is sub memory op id of eid.
       E.g. an opcode gets a value from stack top and changes it.
       This event has two memory ops on the same memory address,
       So we need emid to seq the r/w op, which is an incremental value starting from 1.
    */
    pub emid: u32,
    pub offset: u32,
    pub ltype: LocationType,
    pub atype: AccessType,
    pub vtype: VarType,
    pub is_mutable: bool,
    pub value: u64,
}

impl MemoryTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn is_same_location(&self, other: &MemoryTableEntry) -> bool {
        self.offset == other.offset && self.ltype == other.ltype
    }
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct MTable(Vec<MemoryTableEntry>);

impl MTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn new(entries: Vec<MemoryTableEntry>, imtable: &InitMemoryTable) -> Self {
        let mut mtable = MTable(entries);

        mtable.push_accessed_memory_initialization(imtable);
        mtable.sort();

        mtable
    }

    fn push_accessed_memory_initialization(&mut self, imtable: &InitMemoryTable) {
        let mut set = HashSet::<MemoryTableEntry>::default();

        self.0.iter().for_each(|entry| {
            if entry.ltype == LocationType::Heap || entry.ltype == LocationType::Global {
                let (init_type, value) = match imtable.try_find(entry.ltype, entry.offset) {
                    Some(value) => (InitType::Positive, value),
                    None => (InitType::Lazy, 0),
                };

                set.insert(MemoryTableEntry {
                    eid: 0,
                    emid: 0,
                    offset: entry.offset,
                    ltype: entry.ltype,
                    atype: AccessType::Init(init_type),
                    vtype: entry.vtype,
                    is_mutable: entry.is_mutable,
                    value,
                });
            }
        });

        let mut entries = set.into_iter().collect();

        self.0.append(&mut entries);
    }

    fn sort(&mut self) {
        self.0
            .sort_by_key(|item| (item.ltype, item.offset, item.eid, item.emid))
    }

    pub fn entries(&self) -> &Vec<MemoryTableEntry> {
        &self.0
    }
}
