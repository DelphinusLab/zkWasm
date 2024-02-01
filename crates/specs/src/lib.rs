#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use brtable::BrTable;
use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use etable::EventTableEntry;
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

#[macro_use]
extern crate lazy_static;

pub mod args;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: Arc<InitMemoryTable>,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub static_jtable: Arc<[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER]>,
    pub initialization_state: InitializationState<u32>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ExecutionTable {
    pub etable: Arc<EventTable>,
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
                    vtype: init_memory_entry.vtype,
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

    pub fn write(&self, dir: &PathBuf) {
        fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
            let folder = folder.join(filename);
            let mut fd = File::create(folder.as_path()).unwrap();

            fd.write_all(buf.as_bytes()).unwrap();
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

        write_file(dir, "itable.json", &itable);
        // write_file(&dir, "imtable.json", &imtable);
        write_file(dir, "etable.json", &etable);
        write_file(dir, "jtable.json", &jtable);
        write_file(dir, "external_host_table.json", &external_host_call_table);
    }
}
