use num_bigint::BigUint;

use super::itable::InstructionTableEntry;

#[derive(Debug)]
pub struct JumpTableEntry {
    // caller eid (unique)
    pub eid: u64,
    pub last_jump_eid: u64,
    pub inst: Box<InstructionTableEntry>,
}

impl JumpTableEntry {
    pub fn encode(&self) -> BigUint {
        let mut bn = BigUint::from(self.eid);
        bn = bn << 16;
        bn += self.last_jump_eid;
        bn = bn << 64;
        bn += self.inst.encode_instruction_address();
        bn
    }
}
