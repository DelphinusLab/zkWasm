#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use brtable::BrTable;
use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use etable::EventTableEntry;
use halo2_proofs::pairing::bn256::Fr;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::JumpTable;
use jtable::StaticFrameEntry;
use jtable::STATIC_FRAME_ENTRY_NUMBER;
use mtable::AccessType;
use mtable::LocationType;
use mtable::MTable;
use mtable::MemoryTableEntry;
use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;
use serde::Deserialize;
use serde::Serialize;
use state::InitializationState;
use halo2_proofs::arithmetic::BaseExt;

#[macro_use]
extern crate lazy_static;

pub mod brtable;
pub mod configure_table;
pub mod encode;
pub mod etable;
pub mod external_host_call_table;
pub mod host_function;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod state;
pub mod step;
pub mod types;

#[derive(Default, Serialize, Debug, Clone, Deserialize, PartialEq)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: InitMemoryTable,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub static_jtable: Arc<[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER]>,
    pub initialization_state: InitializationState<u32>,
}

#[derive(Debug, Default, Serialize, Clone, Deserialize, PartialEq)]
pub struct ExecutionTable {
    pub etable: EventTable,
    pub jtable: Arc<JumpTable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable,
    pub post_image_table: CompilationTable,
    pub is_last_slice: bool,
}

impl Tables {
    pub fn default(last_slice_circuit: bool) -> Self {
        Self {
            compilation_tables: CompilationTable::default(),
            execution_tables: ExecutionTable::default(),
            post_image_table: CompilationTable::default(),
            is_last_slice: last_slice_circuit,
        }
    }
}

fn write_file(folder: &PathBuf, filename: &str, buf: &[u8]) {
    std::fs::create_dir_all(folder).unwrap();
    let mut folder = folder.clone();
    folder.push(filename);
    let mut fd = std::fs::File::create(folder.as_path()).unwrap();
    folder.pop();

    fd.write(buf).unwrap();
}

pub enum FileType {
    JSON,
    FLEXBUFFERS,
}

impl Tables {
    pub fn create_memory_table(
        &self,
        memory_event_of_step: fn(&EventTableEntry) -> Vec<MemoryTableEntry>,
    ) -> MTable {
        let mut memory_entries = self
            .execution_tables
            .etable
            .entries()
            .par_iter()
            .map(|entry| memory_event_of_step(entry))
            .collect::<Vec<Vec<_>>>()
            .concat();

        let mut set = HashSet::<MemoryTableEntry>::default();

        memory_entries.iter().for_each(|entry| {
            let init_memory_entry = self
                .compilation_tables
                .imtable
                .try_find(entry.ltype, entry.offset);

            if let Some(init_memory_entry) = init_memory_entry {
                set.insert(MemoryTableEntry {
                    eid: init_memory_entry.eid,
                    offset: entry.offset,
                    ltype: entry.ltype,
                    atype: AccessType::Init,
                    vtype: entry.vtype,
                    is_mutable: entry.is_mutable,
                    value: init_memory_entry.value,
                });
            } else if entry.ltype == LocationType::Heap {
                // Heap value without init memory entry should equal 0
                set.insert(MemoryTableEntry {
                    eid: 0,
                    offset: entry.offset,
                    ltype: entry.ltype,
                    atype: AccessType::Init,
                    vtype: entry.vtype,
                    is_mutable: entry.is_mutable,
                    value: 0,
                });
            }
        });

        memory_entries.append(&mut set.into_iter().collect());

        memory_entries.sort_by_key(|item| (item.ltype, item.offset, item.eid));

        MTable::new(memory_entries)
    }

    pub fn write(&self, dir: Option<PathBuf>, file_type: FileType) {
        let dir = dir.unwrap_or(env::current_dir().unwrap());
        match file_type {
            FileType::JSON => {
                let compilation_table =
                    serde_json::to_string_pretty(&self.compilation_tables).unwrap();
                let execution_table = serde_json::to_string_pretty(&self.execution_tables).unwrap();
                let post_image_table =
                    serde_json::to_string_pretty(&self.post_image_table).unwrap();

                write_file(&dir, "compilation.json", compilation_table.as_bytes());
                write_file(&dir, "execution.json", &execution_table.as_bytes());
                write_file(&dir, "post_image.json", &post_image_table.as_bytes());
            }

            FileType::FLEXBUFFERS => {
                let compilation_tables = flexbuffers::to_vec(&self.compilation_tables).unwrap();
                let execution_tables = flexbuffers::to_vec(&self.execution_tables).unwrap();
                let post_image_table = flexbuffers::to_vec(&self.post_image_table).unwrap();

                write_file(&dir, "compilation.buf", &compilation_tables);
                write_file(&dir, "execution.buf", &execution_tables);
                write_file(&dir, "post_image.buf", &post_image_table);
            }
        }
    }

    pub fn load(dir: PathBuf, is_last_slice: bool, file_type: FileType) -> Tables {
        fn load_file(folder: &PathBuf, filename: &str) -> Vec<u8> {
            let mut folder = folder.clone();
            std::fs::create_dir_all(folder.as_path()).unwrap();
            folder.push(filename);
            let mut file = File::open(folder.as_path()).unwrap();
            let mut buf = vec![];
            file.read_to_end(&mut buf).unwrap();
            buf
        }
        let (compilation_tables, execution_tables, post_image_table) = match file_type {
            FileType::JSON => (
                serde_json::from_slice(load_file(&dir, "compilation.json").as_slice()).unwrap(),
                serde_json::from_slice(load_file(&dir, "execution.json").as_slice()).unwrap(),
                serde_json::from_slice(load_file(&dir, "post_image.json").as_slice()).unwrap(),
            ),
            FileType::FLEXBUFFERS => (
                flexbuffers::from_buffer(&load_file(&dir, "compilation.buf").as_slice()).unwrap(),
                flexbuffers::from_buffer(&load_file(&dir, "execution.buf").as_slice()).unwrap(),
                flexbuffers::from_buffer(&load_file(&dir, "post_image.buf").as_slice()).unwrap(),
            ),
        };

        Tables {
            compilation_tables,
            execution_tables,
            post_image_table,
            is_last_slice,
        }
    }

    pub fn load_instances(instance_path: &PathBuf) -> Vec<Fr> {
        let mut instances = vec![];
        let mut fd = std::fs::File::open(&instance_path).unwrap();
        while let Ok(f) = Fr::read(&mut fd) {
            instances.push(f);
        }

        instances
    }
}
