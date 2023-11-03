#![feature(trait_alias)]
#![deny(unused_imports)]
#![deny(dead_code)]

use std::collections::HashSet;
use std::env;
use std::io::Write;
use std::path::PathBuf;

use brtable::ElemTable;
use configure_table::ConfigureTable;
use etable::EventTable;
use etable::EventTableEntry;
use imtable::InitMemoryTable;
use itable::InstructionTable;
use jtable::JumpTable;
use jtable::StaticFrameEntry;
use mtable::AccessType;
use mtable::LocationType;
use mtable::MTable;
use mtable::MemoryTableEntry;
use rayon::prelude::IntoParallelRefIterator;
use rayon::prelude::ParallelIterator;
use serde::Serialize;

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
pub mod step;
pub mod types;

#[derive(Clone, Debug, Serialize)]
pub struct InitializationState<T> {
    pub eid: T,
    pub fid: T,
    pub iid: T,
    pub frame_id: T,
    pub sp: T,

    pub host_public_inputs: T,
    pub context_in_index: T,
    pub context_out_index: T,
    pub external_host_call_call_index: T,

    pub initial_memory_pages: T,
    pub maximal_memory_pages: T,

    // TODO: open mtable
    // pub rest_mops: Option<T>,
    #[cfg(feature = "continuation")]
    pub jops: T,
}

impl Default for InitializationState<u32> {
    fn default() -> Self {
        Self {
            eid: Default::default(),
            fid: Default::default(),
            iid: Default::default(),
            frame_id: Default::default(),
            sp: Default::default(),

            host_public_inputs: Default::default(),
            context_in_index: Default::default(),
            context_out_index: Default::default(),
            external_host_call_call_index: Default::default(),

            initial_memory_pages: Default::default(),
            maximal_memory_pages: Default::default(),

            #[cfg(feature = "continuation")]
            jops: Default::default(),
        }
    }
}

impl<T: Clone> InitializationState<T> {
    pub fn plain(&self) -> Vec<T> {
        let mut v = vec![];

        v.push(self.eid.clone());
        v.push(self.fid.clone());
        v.push(self.iid.clone());
        v.push(self.frame_id.clone());
        v.push(self.sp.clone());

        v.push(self.host_public_inputs.clone());
        v.push(self.context_in_index.clone());
        v.push(self.context_out_index.clone());
        v.push(self.external_host_call_call_index.clone());

        v.push(self.initial_memory_pages.clone());
        v.push(self.maximal_memory_pages.clone());

        #[cfg(feature = "continuation")]
        v.push(self.jops.clone());

        v
    }

    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> InitializationState<U> {
        InitializationState {
            eid: f(&self.eid),
            fid: f(&self.fid),
            iid: f(&self.iid),
            frame_id: f(&self.frame_id),
            sp: f(&self.sp),

            host_public_inputs: f(&self.host_public_inputs),
            context_in_index: f(&self.context_in_index),
            context_out_index: f(&self.context_out_index),
            external_host_call_call_index: f(&self.external_host_call_call_index),

            initial_memory_pages: f(&self.initial_memory_pages),
            maximal_memory_pages: f(&self.maximal_memory_pages),

            #[cfg(feature = "continuation")]
            jops: f(&self.jops),
        }
    }
}

// TODO: make these tables RC
#[derive(Default, Serialize, Debug, Clone)]
pub struct CompilationTable {
    pub itable: InstructionTable,
    pub imtable: InitMemoryTable,
    pub elem_table: ElemTable,
    pub configure_table: ConfigureTable,
    pub static_jtable: Vec<StaticFrameEntry>,
    pub initialization_state: InitializationState<u32>,
}

#[derive(Default, Serialize, Clone)]
pub struct ExecutionTable {
    pub etable: EventTable,
    pub jtable: JumpTable,
}

#[derive(Default, Clone)]
pub struct Tables {
    pub compilation_tables: CompilationTable,
    pub execution_tables: ExecutionTable,
    pub post_image_table: CompilationTable,
    pub is_last_slice: bool,
}

impl Tables {
    pub fn create_memory_table(
        &self,
        memory_event_of_step: fn(&EventTableEntry, &mut u32) -> Vec<MemoryTableEntry>,
    ) -> MTable {
        let mut memory_entries = self
            .execution_tables
            .etable
            .entries()
            .par_iter()
            .map(|entry| memory_event_of_step(entry, &mut 1))
            .collect::<Vec<Vec<_>>>()
            .concat();

        let init_value = memory_entries
            .par_iter()
            .map(|entry| {
                if entry.ltype == LocationType::Heap || entry.ltype == LocationType::Global {
                    let (_, _, value) = self
                        .compilation_tables
                        .imtable
                        .try_find(entry.ltype, entry.offset)
                        .unwrap();

                    Some(value)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut set = HashSet::<MemoryTableEntry>::default();

        memory_entries
            .iter()
            .zip(init_value.into_iter())
            .for_each(|(entry, init_value)| {
                // If it's heap or global
                if let Some(value) = init_value {
                    set.insert(MemoryTableEntry {
                        eid: 0,
                        emid: 0,
                        offset: entry.offset,
                        ltype: entry.ltype,
                        atype: AccessType::Init,
                        vtype: entry.vtype,
                        is_mutable: entry.is_mutable,
                        value,
                    });
                }
            });

        memory_entries.append(&mut set.into_iter().collect());

        memory_entries.sort_by_key(|item| (item.ltype, item.offset, item.eid, item.emid));

        MTable::new(memory_entries)
    }

    pub fn is_last_slice(&self) -> bool {
        self.is_last_slice
    }
}

impl Tables {
    pub fn write_json(&self, dir: Option<PathBuf>) {
        fn write_file(folder: &PathBuf, filename: &str, buf: &String) {
            let mut folder = folder.clone();
            folder.push(filename);
            let mut fd = std::fs::File::create(folder.as_path()).unwrap();
            folder.pop();

            fd.write(buf.as_bytes()).unwrap();
        }

        let itable = serde_json::to_string_pretty(&self.compilation_tables.itable).unwrap();
        let imtable = serde_json::to_string_pretty(&self.compilation_tables.imtable).unwrap();
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
        write_file(&dir, "imtable.json", &imtable);
        write_file(&dir, "etable.json", &etable);
        write_file(&dir, "jtable.json", &jtable);
        write_file(&dir, "external_host_table.json", &external_host_call_table);
    }
}
