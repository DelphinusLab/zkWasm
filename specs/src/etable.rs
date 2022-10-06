use serde::Serialize;

use crate::{host_function::HostPlugin, step::StepInfo};

use super::itable::InstructionTableEntry;

#[derive(Clone, Debug, Serialize)]
pub struct EventTableEntry {
    pub eid: u64,
    pub sp: u64,
    pub last_jump_eid: u64,
    pub inst: InstructionTableEntry,
    pub step_info: StepInfo,
}

impl EventTableEntry {
    pub fn extra_mops(&self) -> u64 {
        match &self.step_info {
            StepInfo::Return { keep, .. } => 2 * keep.len() as u64,
            StepInfo::Br { keep, .. } => 2 * keep.len() as u64,
            StepInfo::BrIfNez {
                condition, keep, ..
            } => {
                if *condition == 0 {
                    0
                } else {
                    2 * keep.len() as u64
                }
            }
            _ => 0,
        }
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
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

#[derive(Default, Clone, Serialize)]
pub struct EventTable(Vec<EventTableEntry>);

impl EventTable {
    pub fn new(entries: Vec<EventTableEntry>) -> Self {
        Self(entries)
    }

    pub fn entries(&self) -> &Vec<EventTableEntry> {
        &self.0
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
