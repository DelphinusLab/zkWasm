use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct InitMemoryTableEntry {
    pub mmid: u64,
    pub offset: u64,
    /// convert from [u8; 8] via u64::from_le_bytes
    pub value: u64,
}

#[derive(Serialize, Default, Debug)]
pub struct InitMemoryTable(pub Vec<InitMemoryTableEntry>);

impl InitMemoryTable {
    pub fn new(entries: Vec<InitMemoryTableEntry>) -> Self {
        Self(entries)
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }
}
