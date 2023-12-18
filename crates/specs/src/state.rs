use serde::Deserialize;
use serde::Serialize;

use crate::etable::EventTableEntry;
use crate::host_function::HostPlugin;
use crate::imtable::memory_event_of_step;
use crate::imtable::InitMemoryTable;
use crate::imtable::InitMemoryTableEntry;
use crate::itable::Opcode;
use crate::mtable::AccessType;
use crate::step::StepInfo;
use crate::CompilationTable;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    #[cfg(feature = "continuation")]
    pub jops: T,
}

impl<T> InitializationState<T> {
    pub fn field_count() -> usize {
        if cfg!(feature = "continuation") {
            12
        } else {
            11
        }
    }
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

    pub fn for_each<U>(&self, f: impl FnMut(&T) -> U) {
        self.map(f);
    }

    pub fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> InitializationState<U> {
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

pub trait UpdateCompilationTable {
    fn update_init_memory_table(&self, execution_table: &Vec<EventTableEntry>) -> InitMemoryTable;

    fn update_initialization_state(
        &self,
        execution_table: &Vec<EventTableEntry>,
        is_last_slice: bool,
    ) -> InitializationState<u32>;
}

impl UpdateCompilationTable for CompilationTable {
    fn update_init_memory_table(&self, execution_table: &Vec<EventTableEntry>) -> InitMemoryTable {
        // First insert origin imtable entries which may be overwritten.
        let mut map = self.imtable.entries().clone();

        let mut it = execution_table.iter();
        while let Some(etable_entry) = it.next() {
            let memory_writing_entires = memory_event_of_step(etable_entry)
                .into_iter()
                .filter(|entry| entry.atype == AccessType::Write);

            for mentry in memory_writing_entires {
                map.insert(
                    (mentry.ltype, mentry.offset),
                    InitMemoryTableEntry {
                        ltype: mentry.ltype,
                        is_mutable: mentry.is_mutable,
                        offset: mentry.offset,
                        vtype: mentry.vtype,
                        value: mentry.value,
                        eid: etable_entry.eid,
                    },
                );
            }
        }

        InitMemoryTable(map)
    }

    fn update_initialization_state(
        &self,
        execution_table: &Vec<EventTableEntry>,
        is_last_slice: bool,
    ) -> InitializationState<u32> {
        let mut host_public_inputs = self.initialization_state.host_public_inputs;
        let mut context_in_index = self.initialization_state.context_in_index;
        let mut context_out_index = self.initialization_state.context_out_index;
        let mut external_host_call_call_index =
            self.initialization_state.external_host_call_call_index;

        #[cfg(feature = "continuation")]
        let mut jops = self.initialization_state.jops;
        for entry in execution_table {
            match &entry.step_info {
                // TODO: fix hard code
                StepInfo::CallHost {
                    function_name,
                    args,
                    op_index_in_plugin,
                    ..
                } => {
                    if *op_index_in_plugin == HostPlugin::HostInput as usize {
                        if function_name == "wasm_input" && args[0] != 0
                            || function_name == "wasm_output"
                        {
                            host_public_inputs += 1;
                        }
                    } else if *op_index_in_plugin == HostPlugin::Context as usize {
                        if function_name == "wasm_read_context" {
                            context_in_index += 1;
                        } else if function_name == "wasm_write_context" {
                            context_out_index += 1;
                        }
                    }
                }
                StepInfo::ExternalHostCall { .. } => external_host_call_call_index += 1,
                StepInfo::Call { .. } | StepInfo::CallIndirect { .. } | StepInfo::Return { .. } => {
                    #[cfg(feature = "continuation")]
                    {
                        jops += 1;
                    }
                }
                _ => (),
            }
        }

        let last_entry = execution_table.last().unwrap();

        let post_initialization_state = if is_last_slice {
            InitializationState {
                eid: last_entry.eid + 1,
                fid: 0,
                iid: 0,
                frame_id: 0,
                // TODO: why not constant 4095?
                sp: last_entry.sp
                    + if let Opcode::Return { drop, .. } = last_entry.inst.opcode {
                        drop
                    } else {
                        0
                    },

                host_public_inputs,
                context_in_index,
                context_out_index,
                external_host_call_call_index,

                initial_memory_pages: last_entry.allocated_memory_pages,
                maximal_memory_pages: self.configure_table.maximal_memory_pages,

                #[cfg(feature = "continuation")]
                jops,
            }
        } else {
            InitializationState {
                eid: last_entry.eid,
                fid: last_entry.inst.fid,
                iid: last_entry.inst.iid,
                frame_id: last_entry.last_jump_eid,
                sp: last_entry.sp,

                host_public_inputs,
                context_in_index,
                context_out_index,
                external_host_call_call_index,

                initial_memory_pages: last_entry.allocated_memory_pages,
                maximal_memory_pages: self.configure_table.maximal_memory_pages,

                #[cfg(feature = "continuation")]
                jops,
            }
        };

        post_initialization_state
    }
}
