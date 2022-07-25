use serde::Serialize;

#[derive(Serialize)]
pub struct InitMemoryTableEntry {
    pub mmid: u64,
    pub offset: u64,
    pub value: u8,
}

#[derive(Serialize, Default)]
pub struct InitMemoryTable(pub Vec<InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        Self(entries)
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }
}
