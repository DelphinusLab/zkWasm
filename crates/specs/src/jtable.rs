use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

// Inherited frame table entries:
// 1. zkmain
// 2. start(optional)
// 3. active frames inherited from last slice
pub const INHERITED_FRAME_TABLE_ENTRIES: usize = 4096;

const JSON: bool = false;

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct FrameTableEntryInternal {
    // caller eid (unique)
    pub frame_id: u32,
    pub next_frame_id: u32,
    pub callee_fid: u32,
    pub fid: u32,
    pub iid: u32,
    pub returned: bool,
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct InheritedFrameTableEntry(pub Option<FrameTableEntryInternal>);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InheritedFrameEntries(Vec<InheritedFrameTableEntry>);

impl From<Vec<InheritedFrameTableEntry>> for InheritedFrameEntries {
    fn from(value: Vec<InheritedFrameTableEntry>) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct InheritedFrameTable(pub Box<[InheritedFrameTableEntry; INHERITED_FRAME_TABLE_ENTRIES]>);

impl Default for InheritedFrameTable {
    fn default() -> Self {
        Self(Box::new(
            [InheritedFrameTableEntry::default(); INHERITED_FRAME_TABLE_ENTRIES],
        ))
    }
}

impl TryFrom<Vec<InheritedFrameTableEntry>> for InheritedFrameTable {
    type Error = Vec<InheritedFrameTableEntry>;

    fn try_from(value: Vec<InheritedFrameTableEntry>) -> Result<Self, Self::Error> {
        let mut value = value;
        if value.len() > INHERITED_FRAME_TABLE_ENTRIES {
            return Err(value);
        }
        value.resize_with(INHERITED_FRAME_TABLE_ENTRIES, Default::default);
        Ok(Self(Box::new(value.try_into()?)))
    }
}

impl TryFrom<InheritedFrameEntries> for InheritedFrameTable {
    type Error = InheritedFrameEntries;

    fn try_from(value: InheritedFrameEntries) -> Result<Self, Self::Error> {
        if value.0.len() > INHERITED_FRAME_TABLE_ENTRIES {
            return Err(value);
        }

        let mut value = value.0;
        value.resize_with(INHERITED_FRAME_TABLE_ENTRIES, Default::default);
        Ok(Self(Box::new(value.try_into()?)))
    }
}

impl InheritedFrameTable {
    pub fn iter(&self) -> std::slice::Iter<'_, InheritedFrameTableEntry> {
        self.0.iter()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CalledFrameTableEntry(pub FrameTableEntryInternal);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CalledFrameTable(Vec<CalledFrameTableEntry>);

impl CalledFrameTable {
    pub fn new(entries: Vec<CalledFrameTableEntry>) -> Self {
        Self(entries)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CalledFrameTableEntry> {
        self.0.iter()
    }

    pub fn into_inner(self) -> Vec<CalledFrameTableEntry> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
pub struct FrameTable {
    pub inherited: Arc<InheritedFrameEntries>,
    pub called: CalledFrameTable,
}

impl FrameTable {
    pub fn read(path: &PathBuf) -> std::io::Result<Self> {
        let mut fd = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf)?;

        if JSON {
            Ok(serde_json::from_slice(&buf).unwrap())
        } else {
            Ok(bincode::deserialize(&buf).unwrap())
        }
    }

    pub fn write(&self, path: &PathBuf) -> std::io::Result<()> {
        let mut fd = std::fs::File::create(path)?;

        if JSON {
            fd.write_all(serde_json::to_string_pretty(self).unwrap().as_bytes())?;
        } else {
            fd.write_all(&bincode::serialize(self).unwrap())?;
        }
        Ok(())
    }
}
