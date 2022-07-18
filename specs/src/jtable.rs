use super::itable::InstructionTableEntry;
use num_bigint::BigUint;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JumpTableEntry {
    // caller eid (unique)
    pub eid: u64,
    pub last_jump_eid: u64,
    pub inst: Box<InstructionTableEntry>,
}

impl JumpTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn encode(&self) -> BigUint {
        let mut bn = BigUint::from(self.eid);
        bn = bn << 16;
        bn += self.last_jump_eid;
        bn = bn << 64;
        bn += self.inst.encode_instruction_address();
        bn
    }
}
