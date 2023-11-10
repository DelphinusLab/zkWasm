use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::mtable::LocationType;
use crate::mtable::VarType;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct InitMemoryTableEntry {
    pub ltype: LocationType,
    pub is_mutable: bool,
    pub start_offset: u32,
    pub end_offset: u32,
    pub vtype: VarType,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
    pub eid: u32,
}

#[derive(Serialize, Default, Debug, Clone)]
pub struct InitMemoryTable {
    entries: Vec<InitMemoryTableEntry>,
    sorted_global_init_entries: BTreeMap<u32, InitMemoryTableEntry>,
    sorted_stack_init_entries: BTreeMap<u32, InitMemoryTableEntry>,
    sorted_heap_init_entries: Vec<InitMemoryTableEntry>,
}

impl InitMemoryTable {
    pub fn new(mut entries: Vec<InitMemoryTableEntry>) -> Self {
        fn sort(entries: &mut Vec<InitMemoryTableEntry>) {
            entries.sort_by_key(|item| (item.ltype, item.start_offset));
        }

        fn merge(entries: Vec<InitMemoryTableEntry>) -> Vec<InitMemoryTableEntry> {
            let mut merged_entries: Vec<_> = entries
                .iter()
                .filter(|entry| entry.ltype != LocationType::Heap)
                .map(|entry| entry.clone())
                .collect();

            let heap_initial: Vec<_> = entries
                .iter()
                .filter(|entry| entry.ltype == LocationType::Heap)
                .collect();

            if !heap_initial.is_empty() {
                let mut scan = 0;
                let mut cursor = scan + 1;
                while scan < heap_initial.len() && cursor < heap_initial.len() {
                    if heap_initial[scan].value == heap_initial[cursor].value
                        && heap_initial[scan].eid == heap_initial[cursor].eid
                    {
                        cursor += 1;
                    } else {
                        merged_entries.push(InitMemoryTableEntry {
                            ltype: LocationType::Heap,
                            is_mutable: true,
                            start_offset: heap_initial[scan].start_offset,
                            end_offset: heap_initial[cursor - 1].end_offset,
                            vtype: VarType::I64,
                            value: heap_initial[scan].value,
                            eid: heap_initial[scan].eid,
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
                    eid: heap_initial[scan].eid,
                });
            }

            merged_entries
        }

        sort(&mut entries);
        let entries = merge(entries);

        let sorted_heap_init_entries = entries
            .iter()
            .filter(|entry| entry.ltype == LocationType::Heap)
            .map(|entry| entry.clone())
            .collect();
        let sorted_global_init_entries = entries
            .iter()
            .filter(|entry| entry.ltype == LocationType::Global)
            .map(|entry| (entry.start_offset, entry.clone()))
            .collect();
        let sorted_stack_init_entries = entries
            .iter()
            .filter(|entry| entry.ltype == LocationType::Stack)
            .map(|entry| (entry.start_offset, entry.clone()))
            .collect();

        InitMemoryTable {
            entries,
            sorted_global_init_entries,
            sorted_stack_init_entries,
            sorted_heap_init_entries,
        }
    }

    pub fn entries(&self) -> &Vec<InitMemoryTableEntry> {
        &self.entries
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.entries).unwrap()
    }

    pub fn try_find(&self, ltype: LocationType, offset: u32) -> Option<(u32, u32, u32, u64)> {
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
                    self.sorted_heap_init_entries[idx].eid,
                    self.sorted_heap_init_entries[idx].value,
                ));
            }
            LocationType::Global => {
                return self
                    .sorted_global_init_entries
                    .get(&offset)
                    .map(|entry| (entry.start_offset, entry.end_offset, entry.eid, entry.value));
            }
            LocationType::Stack => {
                return self
                    .sorted_stack_init_entries
                    .get(&offset)
                    .map(|entry| (entry.start_offset, entry.end_offset, entry.eid, entry.value));
            }
        }
    }

    pub fn filter(&self, ltype: LocationType) -> Vec<&InitMemoryTableEntry> {
        self.entries.iter().filter(|e| e.ltype == ltype).collect()
    }
}
