use std::collections::HashSet;

use serde::{Serialize, Deserialize};
use strum_macros::EnumIter;

use crate::{imtable::InitMemoryTable, types::Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
pub enum LocationType {
    Heap = 0,
    Stack = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Hash, Eq)]
pub enum AccessType {
    Read = 1,
    Write = 2,
    Init = 3,
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
            _ => todo!(),
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

#[derive(Default, Debug, Serialize, Clone)]
pub struct MTable(Vec<MemoryTableEntry>);

impl MTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn new(
        entries: Vec<MemoryTableEntry>,
        args_of_first_invoking_function: &Vec<Value>,
    ) -> Self {
        let mut mtable = MTable(entries);

        let mut start_sp = 4095;
        for arg in args_of_first_invoking_function {
            mtable.0.push(MemoryTableEntry {
                eid: 0,
                emid: 0,
                mmid: 0, // mmid of stack is always zero
                offset: start_sp,
                ltype: LocationType::Stack,
                atype: AccessType::Init,
                vtype: (*arg).clone().into(),
                value: arg.internal(),
            });

            start_sp -= 1;
        }

        mtable.sort();
        mtable
    }

    pub fn push_accessed_memory_initialization(&mut self, imtable: &InitMemoryTable) {
        let mut set = HashSet::<MemoryTableEntry>::default();

        self.0.iter().for_each(|entry| {
            if entry.ltype == LocationType::Heap {
                let value = imtable.find(entry.mmid, entry.offset);

                set.insert(MemoryTableEntry {
                    eid: 0,
                    emid: 0,
                    mmid: entry.mmid,
                    offset: entry.offset,
                    ltype: entry.ltype,
                    atype: AccessType::Init,
                    vtype: VarType::I64,
                    value,
                });
            }
        });

        let mut entries = set.into_iter().collect();

        self.0.append(&mut entries);
        self.sort()
    }

    fn sort(&mut self) {
        self.0
            .sort_by_key(|item| (item.ltype, item.mmid, item.offset, item.eid, item.emid))
    }

    pub fn entries(&self) -> &Vec<MemoryTableEntry> {
        &self.0
    }
}
