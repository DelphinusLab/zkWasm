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
