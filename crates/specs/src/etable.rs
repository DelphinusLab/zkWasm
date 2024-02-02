use std::alloc::Allocator;

use serde::Deserialize;
use serde::Serialize;

use crate::host_function::HostPlugin;
use crate::itable::InstructionTable;
use crate::itable::InstructionTableEntry;
use crate::step::StepInfo;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventTableEntry {
    pub eid: u32,
    pub fid: u32,
    pub iid: u32,
    pub sp: u32,
    pub allocated_memory_pages: u32,
    pub last_jump_eid: u32,
    pub step_info: StepInfo,
}

impl EventTableEntry {
    pub fn get_instruction<'a>(&self, itable: &'a InstructionTable) -> &'a InstructionTableEntry {
        &itable.get(self.fid, self.iid).as_ref().unwrap()
    }
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

#[derive(Debug)]
pub struct EventTable<A: Allocator + Clone>(Vec<EventTableEntry, A>);

impl<A: Allocator + Default + Clone> Default for EventTable<A> {
    fn default() -> Self {
        Self(Vec::<EventTableEntry, A>::with_capacity_in(0, A::default()))
    }
}

impl<A: Allocator + Serialize + Clone> EventTable<A> {
    pub fn with_capability_in(capacity: usize, alloc: A) -> Self {
        Self(Vec::with_capacity_in(capacity, alloc))
    }

    pub fn new(entries: Vec<EventTableEntry, A>) -> Self {
        Self(entries)
    }

    pub fn unwrap(self) -> Vec<EventTableEntry, A> {
        self.0
    }

    pub fn entries(&self) -> &[EventTableEntry] {
        self.0.as_slice()
    }

    pub fn entries_mut(&mut self) -> &mut Vec<EventTableEntry, A> {
        &mut self.0
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
