use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::itable::InstructionTable;
use crate::itable::InstructionTableEntry;
use crate::step::StepInfo;

const JSON: bool = false;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventTableEntry {
    pub eid: u32,
    pub fid: u32,
    pub iid: u32,
    pub sp: u32,
    pub allocated_memory_pages: u32,
    pub last_jump_eid: u32,
    pub step_info: StepInfo,
}

impl EventTableEntry {
    pub fn get_instruction<'a>(&self, itable: &'a InstructionTable) -> &'a InstructionTableEntry {
        itable.get(self.fid, self.iid)
    }
}

pub struct RestMops {
    rest_mops: Vec<u64>,
}

impl Iterator for RestMops {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.rest_mops.pop()
    }
}

pub struct RestJops {
    rest_jops: Vec<u64>,
}

impl Iterator for RestJops {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.rest_jops.pop()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EventTable(Vec<EventTableEntry>);

impl EventTable {
    pub fn new(entries: Vec<EventTableEntry>) -> Self {
        Self(entries)
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

    pub fn unwrap(self) -> Vec<EventTableEntry> {
        self.0
    }

    pub fn entries(&self) -> &Vec<EventTableEntry> {
        &self.0
    }

    pub fn entries_mut(&mut self) -> &mut Vec<EventTableEntry> {
        &mut self.0
    }
}
