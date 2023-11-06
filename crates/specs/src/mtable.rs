use std::collections::BTreeMap;

use serde::Serialize;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
pub enum LocationType {
    Stack = 1,
    Heap = 2,
    Global = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Hash, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Serialize, Hash, PartialOrd, Ord)]
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

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Hash, Eq, PartialOrd, Ord)]
pub enum MemoryReadSize {
    U8 = 1,
    S8,
    U16,
    S16,
    U32,
    S32,
    I64,
}

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Serialize, Hash, Eq, PartialOrd, Ord)]
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
pub struct MTable(pub Vec<MemoryTableEntry>);

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

#[derive(Clone, Debug, Serialize)]
pub struct MemoryWritingEntry {
    index: usize,
    pub entry: MemoryTableEntry,
    pub end_eid: u32,
}

impl MemoryWritingEntry {
    fn is_same_memory_address(&self, other: &Self) -> bool {
        self.entry.is_same_location(&other.entry)
    }
}

#[derive(Debug, Serialize)]
pub struct MemoryWritingTable(pub Vec<MemoryWritingEntry>);

impl From<MTable> for MemoryWritingTable {
    fn from(value: MTable) -> Self {
        let mut index = 0;

        let mut entries: Vec<MemoryWritingEntry> = value
            .0
            .into_iter()
            .filter_map(|entry| {
                if entry.atype != AccessType::Read {
                    let entry = Some(MemoryWritingEntry {
                        index,
                        entry,
                        end_eid: u32::MAX,
                    });

                    index += 1;

                    entry
                } else {
                    None
                }
            })
            .collect();

        let entries_next = entries.clone();
        let next_iter = entries_next.iter().skip(1);

        entries.iter_mut().zip(next_iter).for_each(|(curr, next)| {
            if curr.is_same_memory_address(next) {
                curr.end_eid = next.entry.eid;
            }
        });

        MemoryWritingTable(entries)
    }
}

impl MemoryWritingTable {
    // (location, offset) |-> Vec<(start_eid, end_eid)>
    pub fn build_lookup_mapping(&self) -> BTreeMap<(LocationType, u32), Vec<(u32, u32)>> {
        let mut mapping = BTreeMap::<_, Vec<(u32, u32)>>::new();

        for entry in &self.0 {
            let ltype = entry.entry.ltype;
            let offset = entry.entry.offset;
            let start_eid = entry.entry.eid;
            let end_eid = entry.end_eid;

            if let Some(entries) = mapping.get_mut(&(ltype, offset)) {
                entries.push((start_eid, end_eid));
            } else {
                mapping.insert((ltype, offset), vec![(start_eid, end_eid)]);
            }
        }

        mapping
    }

    // pub fn write_json(&self, dir: Option<PathBuf>) {
    //     fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
    //         let mut folder = folder.clone();
    //         folder.push(filename);
    //         let mut fd = std::fs::File::create(folder.as_path()).unwrap();
    //         folder.pop();

    //         fd.write(buf.as_bytes()).unwrap();
    //     }

    //     let mtable = serde_json::to_string(self).unwrap();

    //     let dir = dir.unwrap_or(env::current_dir().unwrap());
    //     write_file(&dir, "memory_writing_table.json", &mtable);
    // }
}
