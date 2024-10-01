use std::fmt;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

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

impl Serialize for InheritedFrameTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(INHERITED_FRAME_TABLE_ENTRIES))?;

        for elem in self.0.iter() {
            seq.serialize_element(elem)?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for InheritedFrameTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct InheritedFrameTableVisitor;

        impl<'de> Visitor<'de> for InheritedFrameTableVisitor {
            type Value = InheritedFrameTable;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("InheritedFrameTableEntry")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut entries =
                    Box::new([InheritedFrameTableEntry(None); INHERITED_FRAME_TABLE_ENTRIES]);

                for i in 0..INHERITED_FRAME_TABLE_ENTRIES {
                    if let Some(value) = seq.next_element()? {
                        entries[i] = value;
                    } else {
                        unreachable!()
                    }
                }

                Ok(InheritedFrameTable(entries))
            }
        }

        deserializer.deserialize_seq(InheritedFrameTableVisitor)
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

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let mut fd = std::fs::File::create(path)?;

        if JSON {
            fd.write_all(serde_json::to_string_pretty(self).unwrap().as_bytes())?;
        } else {
            fd.write_all(&bincode::serialize(self).unwrap())?;
        }
        Ok(())
    }
}
