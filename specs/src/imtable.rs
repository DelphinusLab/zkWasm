use serde::Serialize;

#[derive(Serialize)]
pub struct InitMemoryTableEntry {
    pub mmid: u64,
    pub offset: u64,
    pub value: u64,
}

impl InitMemoryTableEntry {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
