use std::cmp::Ordering;

use crate::mtable::LocationType;
use crate::mtable::VarType;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub start_offset: u32,
    pub end_offset: u32,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InitMemoryTable {
    entries: Vec<InitMemoryTableEntry>,
    sorted_global_init_entries: Vec<InitMemoryTableEntry>,
    sorted_heap_init_entries: Vec<InitMemoryTableEntry>,
}

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>, k: u32) -> Self {
        let mut imtable = Self {
            entries: entries
                .into_iter()
                .map(|entry| InitMemoryTableEntry {
                    ltype: entry.ltype,
                    is_mutable: entry.is_mutable,
                    start_offset: entry.start_offset,
                    end_offset: if entry.end_offset == u32::MAX {
                        (1u32 << (k - 1)) - 1
                    } else {
                        entry.end_offset
                    },
                    vtype: entry.vtype,
                    value: entry.value,
                })
                .collect(),
            sorted_global_init_entries: vec![],
            sorted_heap_init_entries: vec![],
        };
        imtable.sort();
        imtable.merge();

        imtable.sorted_heap_init_entries = imtable
            .entries
            .iter()
            .filter(|entry| entry.ltype == LocationType::Heap)
            .map(|entry| entry.clone())
            .collect();
        imtable.sorted_global_init_entries = imtable
            .entries
            .iter()
            .filter(|entry| entry.ltype == LocationType::Global)
            .map(|entry| entry.clone())
            .collect();

        imtable
    }

    pub fn entries(&self) -> &Vec<InitMemoryTableEntry> {
        &self.entries
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.entries).unwrap()
    }

    pub fn try_find(&self, ltype: LocationType, offset: u32) -> Option<(u32, u32, u64)> {
        match ltype {
            LocationType::Heap => {
                let idx = self
                    .sorted_heap_init_entries
                    .binary_search_by(|entry| {
                        if offset >= entry.start_offset && offset <= entry.end_offset {
                            Ordering::Equal
                        } else if offset < entry.start_offset {
                            Ordering::Greater
                        } else {
                            Ordering::Less
                        }
                    })
                    .unwrap();

                return Some((
                    self.sorted_heap_init_entries[idx].start_offset,
                    self.sorted_heap_init_entries[idx].end_offset,
                    self.sorted_heap_init_entries[idx].value,
                ));
            }
            LocationType::Global => {
                for entry in self.sorted_global_init_entries.iter() {
                    if entry.start_offset == offset {
                        return Some((offset, offset, entry.value));
                    }
                }
            }
            LocationType::Stack => unreachable!(),
        }

        None
    }

    fn sort(&mut self) {
        self.entries
            .sort_by_key(|item| (item.ltype, item.start_offset))
    }

    fn merge(&mut self) {
        let mut merged_entries: Vec<_> = self
            .entries()
            .iter()
            .filter(|entry| entry.ltype == LocationType::Global)
            .map(|entry| entry.clone())
            .collect();

        let heap_initial: Vec<_> = self
            .entries()
            .iter()
            .filter(|entry| entry.ltype == LocationType::Heap)
            .collect();

        if !heap_initial.is_empty() {
            let mut scan = 0;
            let mut cursor = scan + 1;
            while scan < heap_initial.len() && cursor < heap_initial.len() {
                if heap_initial[scan].value == heap_initial[cursor].value {
                    cursor += 1;
                } else {
                    merged_entries.push(InitMemoryTableEntry {
                        ltype: LocationType::Heap,
                        is_mutable: true,
                        start_offset: heap_initial[scan].start_offset,
                        end_offset: heap_initial[cursor - 1].end_offset,
                        vtype: VarType::I64,
                        value: heap_initial[scan].value,
                    });

                    scan = cursor;
                    cursor = scan + 1;
                }
            }
            merged_entries.push(InitMemoryTableEntry {
                ltype: LocationType::Heap,
                is_mutable: true,
                start_offset: heap_initial[scan].start_offset,
                end_offset: heap_initial[cursor - 1].end_offset,
                vtype: VarType::I64,
                value: heap_initial[scan].value,
            });
        }

        self.entries = merged_entries;
    }

    pub fn filter(&self, ltype: LocationType) -> Vec<&InitMemoryTableEntry> {
        self.entries.iter().filter(|e| e.ltype == ltype).collect()
    }
}
