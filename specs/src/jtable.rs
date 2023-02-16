use crate::encode::table::encode_frame_table_entry;

use super::itable::InstructionTableEntry;
use num_bigint::{BigUint, ToBigUint};
use serde::Serialize;

// TODO: adapt common range
#[derive(Default, Serialize, Debug, Clone)]
pub struct StaticFrameEntry {
    pub frame_id: u16,
    pub next_frame_id: u16,
    pub callee_fid: u16,
    pub fid: u16,
    pub iid: u16,
}

impl StaticFrameEntry {
    pub fn encode(&self) -> BigUint {
        encode_frame_table_entry(
            self.frame_id.to_biguint().unwrap(),
            self.next_frame_id.to_biguint().unwrap(),
            self.callee_fid.to_biguint().unwrap(),
            self.fid.to_biguint().unwrap(),
            self.iid.to_biguint().unwrap(),
        )
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct JumpTableEntry {
    // caller eid (unique)
    pub eid: u64,
    pub last_jump_eid: u64,
    pub callee_fid: u64,
    pub inst: Box<InstructionTableEntry>,
}

impl JumpTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn encode(&self) -> BigUint {
        encode_frame_table_entry(
            self.eid.to_biguint().unwrap(),
            self.last_jump_eid.to_biguint().unwrap(),
            self.callee_fid.to_biguint().unwrap(),
            self.inst.fid.to_biguint().unwrap(),
            self.inst.iid.to_biguint().unwrap(),
        )
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
