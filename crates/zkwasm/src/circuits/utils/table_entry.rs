use crate::circuits::rtable::common_range_max;
use crate::runtime::memory_event_of_step;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::mtable::MTable;
use specs::mtable::MemoryTableEntry;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;

pub(in crate::circuits) struct MemoryWritingEntry {
    pub(in crate::circuits) entry: MemoryTableEntry,
    pub(in crate::circuits) end_eid: u32,
}

impl MemoryWritingEntry {
    fn is_same_memory_address(&self, other: &Self) -> bool {
        self.entry.is_same_location(&other.entry)
    }
}

pub struct MemoryWritingTable(pub(in crate::circuits) Vec<MemoryWritingEntry>);

impl MemoryWritingTable {
    pub(crate) fn count_rest_memory_finalize_ops(&self) -> (u32, HashSet<(LocationType, u32)>) {
        let mut count = 0u32;
        let mut set = HashSet::default();

        let mut iter = self.0.iter().peekable();

        while let Some(entry) = iter.next() {
            if entry.entry.atype == AccessType::Write
                && iter.peek().map_or(true, |next_entry| {
                    !next_entry.entry.is_same_location(&entry.entry)
                })
            {
                set.insert((entry.entry.ltype, entry.entry.offset));
                count += 1;
            }
        }

        (count, set)
    }

    pub fn from(k: u32, value: MTable) -> Self {
        let maximal_eid = if cfg!(feature = "continuation") {
            u32::MAX
        } else {
            common_range_max(k)
        };

        let mut entries: Vec<MemoryWritingEntry> = value
            .entries()
            .iter()
            .filter_map(|entry| {
                if entry.atype != AccessType::Read {
                    Some(MemoryWritingEntry {
                        entry: entry.clone(),
                        end_eid: maximal_eid,
                    })
                } else {
                    None
                }
            })
            .collect();

        let mut iter = entries.iter_mut().peekable();

        while let Some(entry) = iter.next() {
            if let Some(next_entry) = iter.peek() {
                if entry.is_same_memory_address(next_entry) {
                    entry.end_eid = next_entry.entry.eid;
                }
            }
        }

        // FIXME: create_memory_table pushed a lot of meaningless Stack init. Fix it elegantly.
        let entries = entries
            .into_iter()
            .filter(|entry| entry.entry.eid != entry.end_eid)
            .collect();

        MemoryWritingTable(entries)
    }
}

impl MemoryWritingTable {
    // (location, offset) |-> Vec<(start_eid, end_eid)>
    fn build_lookup_mapping(&self) -> HashMap<(LocationType, u32), Vec<(u32, u32)>> {
        let mut mapping = HashMap::new();

        for entry in &self.0 {
            let ltype = entry.entry.ltype;
            let offset = entry.entry.offset;
            let start_eid = entry.entry.eid;
            let end_eid = entry.end_eid;

            mapping
                .entry((ltype, offset))
                .and_modify(|v: &mut Vec<(u32, u32)>| v.push((start_eid, end_eid)))
                .or_insert(vec![(start_eid, end_eid)]);
        }

        mapping
    }
}

pub struct MemoryRWEntry {
    pub entry: MemoryTableEntry,
    pub start_eid: u32,
    pub end_eid: u32,
}

pub struct EventTableEntryWithMemoryInfo {
    pub eentry: EventTableEntry,
    pub memory_rw_entires: Vec<MemoryRWEntry>,
}

pub(crate) struct EventTableWithMemoryInfo(
    pub(in crate::circuits) Vec<EventTableEntryWithMemoryInfo>,
);

impl EventTableWithMemoryInfo {
    pub(in crate::circuits) fn new(
        event_table: &EventTable,
        memory_writing_table: &MemoryWritingTable,
    ) -> Self {
        let lookup = memory_writing_table.build_lookup_mapping();

        let lookup_mtable_eid = |(eid, ltype, offset, is_writing)| {
            let records = lookup.get(&(ltype, offset)).unwrap();

            if is_writing {
                let idx = records
                    .binary_search_by(|(start_eid, _)| start_eid.cmp(eid))
                    .unwrap();
                records[idx]
            } else {
                let idx = records
                    .binary_search_by(|(start_eid, end_eid)| {
                        if eid <= start_eid {
                            Ordering::Greater
                        } else if eid > end_eid {
                            Ordering::Less
                        } else {
                            Ordering::Equal
                        }
                    })
                    .unwrap();

                records[idx]
            }
        };

        EventTableWithMemoryInfo(
            event_table
                .entries()
                .par_iter()
                .map(|eentry| EventTableEntryWithMemoryInfo {
                    eentry: eentry.clone(),
                    memory_rw_entires: memory_event_of_step(eentry)
                        .iter()
                        .map(|mentry| {
                            let (start_eid, end_eid) = lookup_mtable_eid((
                                &eentry.eid,
                                mentry.ltype,
                                mentry.offset,
                                mentry.atype == AccessType::Write,
                            ));

                            MemoryRWEntry {
                                entry: mentry.clone(),
                                start_eid,
                                end_eid,
                            }
                        })
                        .collect(),
                })
                .collect(),
        )
    }
}
