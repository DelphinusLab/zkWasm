use super::itable::InstructionTableEntry;

pub struct JumpTableEntry {
    pub eid: u64,
    pub last_jump_eid: u64,
    pub inst: Box<InstructionTableEntry>,
}
