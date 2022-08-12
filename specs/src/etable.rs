use serde::Serialize;

use crate::step::StepInfo;

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

    pub fn rest_mops(&self) -> RestMops {
        let mut v = Vec::with_capacity(self.0.len());

        let mut mops_count = self.0.iter().fold(0, |acc, entry| {
            acc + entry.extra_mops() + entry.inst.opcode.mops()
        });

        for entry in self.0.iter() {
            v.push(mops_count);
            mops_count -= entry.extra_mops() + entry.inst.opcode.mops();
        }

        v.reverse();

        RestMops { rest_mops: v }
    }

    pub fn rest_jops(&self) -> RestJops {
        let mut v = Vec::with_capacity(self.0.len());

        // minus 1 becuase the last return is not a jump
        let mut rest_jops = self
            .0
            .iter()
            .fold(0, |acc, entry| acc + entry.inst.opcode.jops())
            - 1;

        for entry in self.0.iter() {
            v.push(rest_jops);

            if rest_jops > 0 {
                rest_jops -= entry.inst.opcode.jops();
            }
        }

        v.reverse();

        RestJops { rest_jops: v }
    }
}
