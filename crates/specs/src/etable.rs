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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EventTable(Vec<EventTableEntry>);

impl EventTable {
    pub fn new(entries: Vec<EventTableEntry>) -> Self {
        Self(entries)
    }

    pub fn unwrap(self) -> Vec<EventTableEntry> {
        self.0
    }

    pub fn entries(&self) -> &Vec<EventTableEntry> {
        &self.0
    }

    pub fn entries_mut(&mut self) -> &mut Vec<EventTableEntry> {
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
