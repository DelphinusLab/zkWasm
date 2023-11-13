use log::error;
use serde::Serialize;
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::mtable::MTable;
use specs::mtable::MemoryTableEntry;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

use crate::circuits::rtable::maximal_common_range;
use crate::runtime::memory_event_of_step;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to lookup memory writing in MemoryWritingTable. It is usually because K is insufficient for the execution, please increase K and try again.")]
    CircuitSizeNotEnough,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::circuits) struct MemoryWritingEntry {
    index: usize,
    pub(in crate::circuits) entry: MemoryTableEntry,
    pub(in crate::circuits) end_eid: u32,
}

impl MemoryWritingEntry {
    fn is_same_memory_address(&self, other: &Self) -> bool {
        self.entry.is_same_location(&other.entry)
    }
}

#[derive(Debug, Serialize)]
pub struct MemoryWritingTable(pub(in crate::circuits) Vec<MemoryWritingEntry>);

impl From<MTable> for MemoryWritingTable {
    fn from(value: MTable) -> Self {
        let maximal_eid = maximal_common_range();
        let mut index = 0;

        let mut entries: Vec<MemoryWritingEntry> = value
            .entries()
            .iter()
            .filter_map(|entry| {
                if entry.atype != AccessType::Read {
                    let entry = Some(MemoryWritingEntry {
                        index,
                        entry: entry.clone(),
                        end_eid: maximal_eid,
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
    fn build_lookup_mapping(&self) -> BTreeMap<(LocationType, u32), Vec<(u32, u32)>> {
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

    pub fn write_json(&self, dir: Option<PathBuf>) {
        fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
            let mut folder = folder.clone();
            folder.push(filename);
            let mut fd = std::fs::File::create(folder.as_path()).unwrap();
            folder.pop();

            fd.write(buf.as_bytes()).unwrap();
        }

        let mtable = serde_json::to_string_pretty(self).unwrap();

        let dir = dir.unwrap_or(env::current_dir().unwrap());
        write_file(&dir, "memory_writing_table.json", &mtable);
    }
}

#[derive(Debug)]
pub struct MemoryRWEntry {
    pub entry: MemoryTableEntry,
    pub start_eid: u32,
    pub end_eid: u32,
}

#[derive(Debug)]
pub struct EventTableEntryWithMemoryInfo {
    pub eentry: EventTableEntry,
    pub memory_rw_entires: Vec<MemoryRWEntry>,
}

#[derive(Debug)]
pub(crate) struct EventTableWithMemoryInfo(
    pub(in crate::circuits) Vec<EventTableEntryWithMemoryInfo>,
);

impl EventTableWithMemoryInfo {
    pub(in crate::circuits) fn new(
        event_table: &EventTable,
        memory_writing_table: &MemoryWritingTable,
    ) -> Result<Self, Error> {
        let lookup = memory_writing_table.build_lookup_mapping();

        let lookup_mtable_eid = |(eid, ltype, offset, is_writing)| -> Result<(u32, u32), Error> {
            let records = lookup.get(&(ltype, offset)).unwrap();

            // The MemoryWritingTable uses maximal_common_range as maximal end eid, hence
            // searching will be failed if eid exceeds maximal_common_range.
            if eid > maximal_common_range() {
                error!(
                    "eid {} exceeds the maximal common range {}",
                    eid,
                    maximal_common_range()
                );

                return Err(Error::CircuitSizeNotEnough);
            }

            if is_writing {
                let idx = records.binary_search_by(|(start_eid, _)| start_eid.cmp(&eid));

                Ok(records[idx.unwrap()])
            } else {
                let idx = records.binary_search_by(|(start_eid, end_eid)| {
                    if eid <= *start_eid {
                        Ordering::Greater
                    } else if eid > *end_eid {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                });

                Ok(records[idx.unwrap()])
            }
        };

        let entries = event_table
            .entries()
            .iter()
            .map(|eentry| {
                let memory_rw_entires = memory_event_of_step(eentry, &mut 1)
                    .into_iter()
                    .map(|mentry| {
                        let (start_eid, end_eid) = lookup_mtable_eid((
                            eentry.eid,
                            mentry.ltype,
                            mentry.offset,
                            mentry.atype == AccessType::Write,
                        ))?;

                        Ok(MemoryRWEntry {
                            entry: mentry.clone(),
                            start_eid,
                            end_eid,
                        })
                    })
                    .collect::<Result<_, _>>()?;

                Ok(EventTableEntryWithMemoryInfo {
                    eentry: eentry.clone(),
                    memory_rw_entires,
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(EventTableWithMemoryInfo(entries))
    }
}
