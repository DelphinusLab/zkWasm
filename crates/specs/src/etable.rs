use serde::Serialize;

use super::itable::InstructionTableEntry;
use crate::host_function::HostPlugin;
use crate::mtable::AccessType;
use crate::mtable::MemoryWritingTable;
use crate::step::StepInfo;

#[derive(Clone, Debug, Serialize)]
pub struct EventTableEntry {
    pub eid: u32,
    pub sp: u32,
    pub allocated_memory_pages: u32,
    pub last_jump_eid: u32,
    pub inst: InstructionTableEntry,
    pub step_info: StepInfo,
}

pub struct RestMops {
    rest_mops: Vec<u64>,
}

impl Iterator for RestMops {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.rest_mops.pop()
    }
}

pub struct RestJops {
    rest_jops: Vec<u64>,
}

impl Iterator for RestJops {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.rest_jops.pop()
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct EventTable(Vec<EventTableEntry>);

impl EventTable {
    pub fn new(entries: Vec<EventTableEntry>) -> Self {
        Self(entries)
    }

    pub fn entries(&self) -> &Vec<EventTableEntry> {
        &self.0
    }

    pub fn entries_mut(&mut self) -> &mut Vec<EventTableEntry> {
        &mut self.0
    }

    pub fn unwrap(self) -> Vec<EventTableEntry> {
        self.0
    }

    pub fn filter_foreign_entries(&self, foreign: HostPlugin) -> Vec<EventTableEntry> {
        self.0
            .clone()
            .into_iter()
            .filter(|entry| match entry.step_info {
                StepInfo::CallHost { plugin, .. } => plugin == foreign,
                _ => false,
            })
            .collect::<Vec<_>>()
    }
}

use std::cmp::Ordering;

use crate::mtable::MemoryTableEntry;

#[derive(Debug)]
pub struct MemoryRWEntry {
    pub entry: MemoryTableEntry,
    pub start_eid: u32,
    pub end_eid: u32,
}

#[derive(Debug)]
pub struct EventTableEntryWithMemoryInfo<'a> {
    pub eentry: &'a EventTableEntry,
    pub memory_rw_entires: Vec<MemoryRWEntry>,
}

#[derive(Debug)]
pub struct EventTableWithMemoryInfo<'a>(pub Vec<EventTableEntryWithMemoryInfo<'a>>);

impl<'a> EventTableWithMemoryInfo<'a> {
    pub fn new(
        event_table: &'a EventTable,
        memory_writing_table: &MemoryWritingTable,
        memory_event_of_step: fn(&EventTableEntry, &mut u32) -> Vec<MemoryTableEntry>,
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
                .iter()
                .map(|eentry| EventTableEntryWithMemoryInfo {
                    eentry,
                    memory_rw_entires: memory_event_of_step(eentry, &mut 1)
                        .into_iter()
                        .map(|mentry| {
                            let (start_eid, end_eid) = lookup_mtable_eid((
                                &eentry.eid,
                                mentry.ltype,
                                mentry.offset,
                                mentry.atype == AccessType::Write,
                            ));

                            MemoryRWEntry {
                                entry: mentry,
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
