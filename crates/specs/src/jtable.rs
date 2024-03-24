use serde::Deserialize;
use serde::Serialize;

// 1. jumps to zkmain
// 2. jumps to start(if exists)
pub const STATIC_FRAME_ENTRY_NUMBER: usize = 2;

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct StaticFrameEntry {
    pub enable: bool,
    pub frame_id: u32,
    pub next_frame_id: u32,
    pub callee_fid: u32,
    pub fid: u32,
    pub iid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JumpTableEntry {
    // caller eid (unique)
    pub eid: u32,
    pub last_jump_eid: u32,
    pub callee_fid: u32,
    pub fid: u32,
    pub iid: u32,
}

impl JumpTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JumpTable(Vec<JumpTableEntry>);

impl JumpTable {
    pub fn entries(&self) -> &Vec<JumpTableEntry> {
        &self.0
    }

    pub fn push(&mut self, entry: JumpTableEntry) {
        self.0.push(entry)
    }
}
