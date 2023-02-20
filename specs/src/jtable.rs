use crate::{encode::frame_table::encode_frame_table_entry, utils::common_range::CommonRange};

use super::itable::InstructionTableEntry;
use num_bigint::BigUint;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct StaticFrameEntry {
    pub frame_id: CommonRange,
    pub next_frame_id: CommonRange,
    pub callee_fid: CommonRange,
    pub fid: CommonRange,
    pub iid: CommonRange,
}

impl StaticFrameEntry {
    pub fn encode(&self) -> BigUint {
        encode_frame_table_entry(
            self.frame_id.into(),
            self.next_frame_id.into(),
            self.callee_fid.into(),
            self.fid.into(),
            self.iid.into(),
        )
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct JumpTableEntry {
    // caller eid (unique)
    pub eid: CommonRange,
    pub last_jump_eid: CommonRange,
    pub callee_fid: CommonRange,
    pub inst: Box<InstructionTableEntry>,
}

impl JumpTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn encode(&self) -> BigUint {
        encode_frame_table_entry(
            self.eid.into(),
            self.last_jump_eid.into(),
            self.callee_fid.into(),
            self.inst.fid.into(),
            self.inst.iid.into(),
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
