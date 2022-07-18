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
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
