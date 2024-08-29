#![feature(trait_alias)]
#![deny(warnings)]
#![allow(
    clippy::assertions_on_constants,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use brtable::BrTable;
use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::FrameTable;
use jtable::InheritedFrameTable;
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
    File {
        event_table_writer: Box<dyn Fn(usize, &EventTable) -> PathBuf>,
        frame_table_writer: Box<dyn Fn(usize, &FrameTable) -> PathBuf>,
    },
    Memory,
}

pub enum TableBackend<Table> {
    Memory(Table),
    Json(PathBuf),
}

#[derive(Debug)]
pub struct CompilationTable {
    pub itable: Arc<InstructionTable>,
    pub imtable: Arc<InitMemoryTable>,
    pub br_table: Arc<BrTable>,
    pub elem_table: Arc<ElemTable>,
    pub configure_table: Arc<ConfigureTable>,
    pub initial_frame_table: Arc<InheritedFrameTable>,
    pub initialization_state: Arc<InitializationState<u32>>,
}

#[derive(Default)]
pub struct ExecutionTable {
    pub etable: Vec<TableBackend<EventTable>>,
    pub frame_table: Vec<TableBackend<FrameTable>>,
    pub external_host_call_table: Vec<ExternalHostCallTable>,
    pub context_input_table: Vec<u64>,
    pub context_output_table: Vec<u64>,
}

pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable,
}

impl Tables {
    pub fn write(&self, dir: &Path, name_of_frame_table_slice: impl Fn(usize) -> String) {
        fn write_file(folder: &Path, filename: &str, buf: &String) {
            let folder = folder.join(filename);
            let mut fd = File::create(folder.as_path()).unwrap();

            fd.write_all(buf.as_bytes()).unwrap();
        }

        write_file(
            dir,
            "itable.json",
            &serde_json::to_string_pretty(&self.compilation_tables.itable).unwrap(),
        );
        self.execution_tables
            .frame_table
            .iter()
            .enumerate()
            .for_each(|(slice, frame_table)| {
                if let TableBackend::Memory(frame_table) = frame_table {
                    write_file(
                        dir,
                        &name_of_frame_table_slice(slice),
                        &serde_json::to_string_pretty(frame_table).unwrap(),
                    );
                }
            });

        for (i, external_host_call_table) in self
            .execution_tables
            .external_host_call_table
            .iter()
            .enumerate()
        {
            write_file(
                dir,
                &format!("external_host_table.{}.json", i),
                &serde_json::to_string_pretty(&external_host_call_table).unwrap(),
            );
        }
    }
}
