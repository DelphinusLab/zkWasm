use super::itable::InstructionTableEntry;
use num_bigint::BigUint;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
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
        bn = bn << 32;
        bn += self.inst.encode_instruction_address();
        bn
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct JumpTable(Vec<JumpTableEntry>);

impl JumpTable {
    pub fn entries(&self) -> &Vec<JumpTableEntry> {
        &self.0
    }

    pub fn push(&mut self, entry: JumpTableEntry) {
        self.0.push(entry)
    }
}
