use super::itable::InstructionTableEntry;

#[derive(Debug)]
pub struct JumpTableEntry {
    // caller eid (unique)
    pub eid: u64,
    pub last_jump_eid: u64,
    pub inst: Box<InstructionTableEntry>,
}
