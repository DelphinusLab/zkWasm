use std::{cmp::Ordering, collections::BTreeMap};

use crate::mtable::{LocationType, VarType};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct InitMemoryEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub mmid: u64,
    pub offset: u64,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ImportMemoryEntry {
    pub ltype: LocationType,
    pub origin_moid: u16,
    pub origin_idx: u16,
    pub moid: u16,
    pub idx: u16,
}

#[derive(Serialize, Debug, Clone)]
pub enum InitMemoryTableEntry {
    Import(ImportMemoryEntry),
    Init(InitMemoryEntry),
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct InitMemoryTable(pub Vec<InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        let mut imtable = Self(entries);
        imtable.sort();
        imtable
    }

    pub fn push(&mut self, entry: InitMemoryTableEntry) {
        self.0.push(entry)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }

    pub fn find_init(&self, ltype: LocationType, mmid: u64, offset: u64) -> u64 {
        for entry in self.0.iter() {
            match entry {
                InitMemoryTableEntry::Init(entry) => {
                    if entry.ltype == ltype && entry.mmid == mmid && entry.offset == offset {
                        return entry.value;
                    }
                }
                InitMemoryTableEntry::Import(_) => (),
            }
        }

        unreachable!()
    }

    fn sort(&mut self) {
        //self.0
        //.sort_by_key(|item| (item.index(), item.ltype, item.mmid, item.offset))
        self.0.sort_by(|a, b| match (a, b) {
            (InitMemoryTableEntry::Import(_), InitMemoryTableEntry::Import(_)) => Ordering::Equal,
            (InitMemoryTableEntry::Import(_), InitMemoryTableEntry::Init(_)) => Ordering::Less,
            (InitMemoryTableEntry::Init(_), InitMemoryTableEntry::Import(_)) => Ordering::Greater,
            (InitMemoryTableEntry::Init(a), InitMemoryTableEntry::Init(b)) => a
                .ltype
                .cmp(&b.ltype)
                .then(a.mmid.cmp(&b.mmid))
                .then(a.offset.cmp(&b.offset)),
        })
    }

    fn filter_init(&self, ltype: LocationType) -> Vec<&InitMemoryEntry> {
        let mut entries = vec![];

        for e in &self.0 {
            match e {
                InitMemoryTableEntry::Import(_) => (),
                InitMemoryTableEntry::Init(e) => {
                    if e.ltype == ltype {
                        entries.push(e)
                    }
                }
            }
        }

        entries
    }

    pub fn filter_memory_init(&self) -> Vec<&InitMemoryEntry> {
        self.filter_init(LocationType::Heap)
    }

    pub fn filter_global_init(&self) -> Vec<Vec<&InitMemoryEntry>> {
        let entries = self.filter_init(LocationType::Global);

        let mut map = BTreeMap::new();

        for e in entries {
            if map.contains_key(&e.mmid) {
                let group: &mut Vec<&InitMemoryEntry> = map.get_mut(&e.mmid).unwrap();
                group.push(e)
            } else {
                map.insert(e.mmid, vec![e]);
            }
        }

        let mut ret = vec![];

        for (_, group) in map {
            ret.push(group)
        }

        ret
    }

    pub fn filter_import(&self) -> Vec<&ImportMemoryEntry> {
        let mut entries = vec![];

        for e in &self.0 {
            match e {
                InitMemoryTableEntry::Import(e) => entries.push(e),
                InitMemoryTableEntry::Init(_) => (),
            }
        }

        entries
    }
}
