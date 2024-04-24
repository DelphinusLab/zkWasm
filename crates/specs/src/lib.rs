#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use brtable::BrTable;
use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use etable::EventTableBackend;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::JumpTable;
use jtable::StaticFrameEntry;
use jtable::STATIC_FRAME_ENTRY_NUMBER;
use num_bigint::BigUint;
use serde::Deserialize;
use serde::Serialize;
use state::InitializationState;

use crate::external_host_call_table::ExternalHostCallTable;

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
pub mod slice;
pub mod state;
pub mod step;
pub mod types;

pub enum TraceBackend {
    File(Box<dyn Fn(usize, &EventTable) -> PathBuf>),
    Memory,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: Arc<InitMemoryTable>,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub static_jtable: Arc<[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER]>,
    pub initialization_state: Arc<InitializationState<u32, BigUint>>,
}

#[derive(Default)]
pub struct ExecutionTable {
    pub etable: Vec<EventTableBackend>,
    pub jtable: JumpTable,
}

pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable,
}

impl Tables {
    pub fn write(&self, dir: &PathBuf, name_of_etable_slice: impl Fn(usize) -> String) {
        fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
            let folder = folder.join(filename);
            let mut fd = File::create(folder.as_path()).unwrap();

            fd.write_all(buf.as_bytes()).unwrap();
        }

        let mut external_host_call_table = vec![];
        self.execution_tables
            .etable
            .iter()
            .enumerate()
            .for_each(|(slice, e)| match e {
                EventTableBackend::Memory(etable) => {
                    external_host_call_table.extend(etable.filter_external_host_call_table().0);
                    write_file(
                        dir,
                        &name_of_etable_slice(slice),
                        &serde_json::to_string_pretty(etable).unwrap(),
                    );
                }
                EventTableBackend::Json(_) => {}
            });
        let external_host_call_table = ExternalHostCallTable::new(external_host_call_table);

        write_file(
            dir,
            "itable.json",
            &serde_json::to_string_pretty(&self.compilation_tables.itable).unwrap(),
        );
        write_file(
            dir,
            "jtable.json",
            &serde_json::to_string_pretty(&self.execution_tables.jtable).unwrap(),
        );
        write_file(
            dir,
            "external_host_table.json",
            &serde_json::to_string_pretty(&external_host_call_table).unwrap(),
        );
    }
}
