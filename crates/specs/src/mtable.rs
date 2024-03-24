use serde::Deserialize;
use serde::Serialize;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum LocationType {
    Stack = 1,
    Heap = 2,
    Global = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub enum AccessType {
    Read = 1,
    Write = 2,
    Init = 3,
}

impl AccessType {
    pub fn is_init(&self) -> bool {
        *self == AccessType::Init
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, Serialize, Deserialize, Hash, PartialOrd, Ord,
)]
pub enum VarType {
    I64 = 0,
    I32 = 1,
}

impl VarType {
    pub fn byte_size(&self) -> u64 {
        match self {
            VarType::I32 => 4,
            VarType::I64 => 8,
        }
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord,
)]
pub enum MemoryReadSize {
    U8 = 1,
    S8,
    U16,
    S16,
    U32,
    S32,
    I64,
}

#[derive(
    Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord,
)]
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
    pub fn byte_size(&self) -> u32 {
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

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct MemoryTableEntry {
    pub eid: u32,
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

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct MTable(Vec<MemoryTableEntry>);

impl MTable {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn entries(&self) -> &Vec<MemoryTableEntry> {
        &self.0
    }

    pub(crate) fn new(entries: Vec<MemoryTableEntry>) -> MTable {
        MTable(entries)
    }
}
