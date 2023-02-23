use std::collections::BTreeMap;

use specs::{
    etable::{EventTable, EventTableEntry},
    mtable::{AccessType, LocationType, MTable, MemoryTableEntry},
};

use crate::{circuits::config::zkwasm_k, runtime::memory_event_of_step};

#[derive(Clone)]
pub(in crate::circuits) struct MemoryWritingEntry {
    pub(in crate::circuits) entry: MemoryTableEntry,
    pub(in crate::circuits) end_eid: u32,
}

impl MemoryWritingEntry {
    fn is_same_memory_address(&self, other: &Self) -> bool {
        self.entry.is_same_location(&other.entry)
    }
}

pub(in crate::circuits) struct MemoryWritingTable(pub(in crate::circuits) Vec<MemoryWritingEntry>);

impl From<MTable> for MemoryWritingTable {
    fn from(value: MTable) -> Self {
        let maximal_eid = (1u32 << (zkwasm_k() - 1)) - 1;

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
}

pub(in crate::circuits) struct MemoryReadingEntry {
    entry: MemoryTableEntry,
    start_eid: u32,
    end_eid: u32,
}

pub(in crate::circuits) struct EventTableEntryWithMemoryReading {
    pub(crate) eentry: EventTableEntry,
    pub(crate) memory_read_entires: Vec<MemoryReadingEntry>,
}

pub(crate) struct EventTableEntryWithMemoryReadingTable(
    pub(in crate::circuits) Vec<EventTableEntryWithMemoryReading>,
);

impl EventTableEntryWithMemoryReadingTable {
    pub(in crate::circuits) fn new(
        event_table: EventTable,
        memory_writing_table: &MemoryWritingTable,
    ) -> Self {
        let lookup = memory_writing_table.build_lookup_mapping();

        let lookup_mtable_eid = |(eid, ltype, offset)| {
            let records = lookup.get(&(ltype, offset)).unwrap();

            records
                .iter()
                .find(|(start_eid, end_eid)| *start_eid <= eid && eid < *end_eid)
                .unwrap()
        };

        EventTableEntryWithMemoryReadingTable(
            event_table
                .entries()
                .iter()
                .map(|eentry| EventTableEntryWithMemoryReading {
                    eentry: eentry.clone(),
                    memory_read_entires: memory_event_of_step(eentry, &mut 1)
                        .iter()
                        .filter_map(|mentry| {
                            if mentry.atype == AccessType::Read {
                                let (start_eid, end_eid) =
                                    lookup_mtable_eid((eentry.eid, mentry.ltype, mentry.offset));

                                Some(MemoryReadingEntry {
                                    entry: mentry.clone(),
                                    start_eid: *start_eid,
                                    end_eid: *end_eid,
                                })
                            } else {
                                None
                            }
                        })
                        .collect(),
                })
                .collect(),
        )
    }
}
