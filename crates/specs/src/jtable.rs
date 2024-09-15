use std::collections::HashMap;
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
pub struct InheritedFrameTableEntry(pub FrameTableEntryInternal);

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InheritedFrameTable(Arc<Vec<InheritedFrameTableEntry>>);

impl InheritedFrameTable {
    pub fn new(entries: Vec<InheritedFrameTableEntry>) -> Self {
        Self(Arc::new(entries))
    }
}

pub struct InheritedFrameTableIter {
    table: Arc<Vec<InheritedFrameTableEntry>>,
    cursor: usize,
}

impl Iterator for InheritedFrameTableIter {
    type Item = Option<InheritedFrameTableEntry>;

    /*
     * None: should stop
     * Some(None): disabled inherited entry
     * Some(Some(_)): enabled inherited entry
     */
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor == INHERITED_FRAME_TABLE_ENTRIES {
            return None;
        }

        let r = self.table.get(self.cursor).cloned();

        self.cursor += 1;

        Some(r)
    }
}

impl IntoIterator for InheritedFrameTable {
    type Item = Option<InheritedFrameTableEntry>;

    type IntoIter = InheritedFrameTableIter;

    fn into_iter(self) -> Self::IntoIter {
        assert!(self.0.len() <= INHERITED_FRAME_TABLE_ENTRIES);

        InheritedFrameTableIter {
            table: self.0,
            cursor: 0,
        }
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
    pub inherited: InheritedFrameTable,
    pub called: CalledFrameTable,
}

impl FrameTable {
    pub fn build_returned_lookup_mapping(&self) -> HashMap<(u32, u32), bool> {
        let mut lookup_table = HashMap::with_capacity(self.called.len() + self.inherited.0.len());
        for entry in self.called.iter() {
            lookup_table.insert((entry.0.frame_id, entry.0.callee_fid), entry.0.returned);
        }
        for entry in self.inherited.0.iter() {
            lookup_table.insert((entry.0.frame_id, entry.0.callee_fid), entry.0.returned);
        }

        lookup_table
    }

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
