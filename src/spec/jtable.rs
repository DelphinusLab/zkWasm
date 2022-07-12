use super::itable::InstructionTableEntry;

pub struct JumpTableEntry {
    eid: u64,
    last_jump_eid: u64,
    inst: Box<InstructionTableEntry>,
}
