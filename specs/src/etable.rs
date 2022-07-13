use crate::step::StepInfo;

use super::itable::InstructionTableEntry;

#[derive(Clone, Debug)]
pub struct EventTableEntry {
    pub eid: u64,
    pub sp: u64,
    pub last_jump_eid: u64,
    pub inst: InstructionTableEntry,
    pub step_info: StepInfo,
}
