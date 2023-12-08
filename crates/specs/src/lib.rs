#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use etable::EventTableEntry;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::JumpTable;
use jtable::StaticFrameEntry;
use mtable::AccessType;
use mtable::MTable;
use mtable::MemoryTableEntry;
use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;
use serde::Deserialize;
use serde::Serialize;
use state::InitializationState;

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

#[derive(Default, Serialize, Debug, Clone, Deserialize)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: InitMemoryTable,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub static_jtable: Arc<Vec<StaticFrameEntry>>,
    pub initialization_state: InitializationState<u32>,
}

#[derive(Default, Serialize, Clone, Deserialize)]
pub struct ExecutionTable {
    pub etable: EventTable,
    pub jtable: Arc<JumpTable>,
}

#[derive(Clone)]
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

        let init_value = memory_entries
            .par_iter()
            .map(|entry| {
                self.compilation_tables
                    .imtable
                    .try_find(entry.ltype, entry.offset)
            })
            .collect::<Vec<_>>();

        let mut set = HashSet::<MemoryTableEntry>::default();

        memory_entries
            .iter()
            .zip(init_value.into_iter())
            .for_each(|(entry, init_memory_entry)| {
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
                }
            });

        memory_entries.append(&mut set.into_iter().collect());

        memory_entries.sort_by_key(|item| (item.ltype, item.offset, item.eid));

        MTable::new(memory_entries)
    }

    pub fn write_json(&self, dir: Option<PathBuf>) {
        fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
            let mut folder = folder.clone();
            std::fs::create_dir_all(folder.as_path()).unwrap();
            folder.push(filename);
            let mut fd = std::fs::File::create(folder.as_path()).unwrap();
            folder.pop();

            fd.write(buf.as_bytes()).unwrap();
        }

        let compilation_table = serde_json::to_string_pretty(&self.compilation_tables).unwrap();
        let execution_table = serde_json::to_string_pretty(&self.execution_tables).unwrap();
        let post_image_table = serde_json::to_string_pretty(&self.post_image_table).unwrap();

        println!("dir path: {:?}", dir);
        let dir = dir.unwrap_or(env::current_dir().unwrap());
        println!("dir path unrap: {:?}", dir);
        write_file(&dir, "compilation.json", &compilation_table);
        write_file(&dir, "execution.json", &execution_table);
        write_file(&dir, "image.json", &post_image_table);
    }

    pub fn load_json(dir: PathBuf, is_last_slice: bool) -> Tables {
        fn load_file(folder: &PathBuf, filename: &str) -> BufReader<File> {
            let mut folder = folder.clone();
            folder.push(filename);
            let file = File::open(folder.as_path()).unwrap();
            BufReader::new(file)
        }

        let compilation_tables: CompilationTable =
            serde_json::from_reader(load_file(&dir, "compilation.json")).unwrap();
        let execution_tables: ExecutionTable =
            serde_json::from_reader(load_file(&dir, "execution.json")).unwrap();
        let post_image_table: CompilationTable =
            serde_json::from_reader(load_file(&dir, "image.json")).unwrap();
        Tables {
            compilation_tables,
            execution_tables,
            post_image_table,
            is_last_slice,
        }
    }
}
