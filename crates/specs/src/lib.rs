#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::collections::HashSet;
use std::env;
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

#[derive(Default, Serialize, Debug, Clone)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: InitMemoryTable,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub static_jtable: Arc<Vec<StaticFrameEntry>>,
    pub initialization_state: InitializationState<u32>,
}

#[derive(Default, Serialize, Clone)]
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
            folder.push(filename);
            let mut fd = std::fs::File::create(folder.as_path()).unwrap();
            folder.pop();

            fd.write(buf.as_bytes()).unwrap();
        }

        let itable = serde_json::to_string_pretty(&self.compilation_tables.itable).unwrap();
        // let imtable = serde_json::to_string_pretty(&self.compilation_tables.imtable).unwrap();
        let etable = serde_json::to_string_pretty(&self.execution_tables.etable).unwrap();
        let external_host_call_table = serde_json::to_string_pretty(
            &self
                .execution_tables
                .etable
                .filter_external_host_call_table(),
        )
        .unwrap();
        let jtable = serde_json::to_string_pretty(&self.execution_tables.jtable).unwrap();

        let dir = dir.unwrap_or(env::current_dir().unwrap());
        write_file(&dir, "itable.json", &itable);
        // write_file(&dir, "imtable.json", &imtable);
        write_file(&dir, "etable.json", &etable);
        write_file(&dir, "jtable.json", &jtable);
        write_file(&dir, "external_host_table.json", &external_host_call_table);
    }
}
