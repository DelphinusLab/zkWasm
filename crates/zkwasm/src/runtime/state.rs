use specs::etable::EventTableEntry;
use specs::host_function::HostPlugin;
use specs::imtable::InitMemoryTable;
use specs::imtable::InitMemoryTableEntry;
use specs::mtable::AccessType;
use specs::state::InitializationState;
use specs::step::StepInfo;
use specs::CompilationTable;

use super::memory_event_of_step;

pub(crate) trait UpdateCompilationTable {
    fn update_init_memory_table(&self, execution_table: &Vec<EventTableEntry>) -> InitMemoryTable;

    fn update_initialization_state(
        &self,
        execution_table: &Vec<EventTableEntry>,
        next_event_entry: Option<&EventTableEntry>,
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
        // None indicates last slice
        next_event_entry: Option<&EventTableEntry>,
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
                    plugin,
                    function_name,
                    args,
                    ..
                } => {
                    if *plugin == HostPlugin::HostInput {
                        if (function_name == "wasm_input" && args[0] != 0)
                            || function_name == "wasm_output"
                        {
                            host_public_inputs += 1;
                        }
                    } else if *plugin == HostPlugin::Context {
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

        let post_initialization_state = if next_event_entry.is_none() {
            let last_entry = execution_table.last().unwrap();

            InitializationState {
                eid: last_entry.eid + 1,
                fid: 0,
                iid: 0,
                frame_id: 0,
                // TODO: why not constant 4095?
                sp: last_entry.sp
                    + if let StepInfo::Return { drop, .. } = last_entry.step_info {
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
            let next_entry = next_event_entry.unwrap();

            InitializationState {
                eid: next_entry.eid,
                fid: next_entry.fid,
                iid: next_entry.iid,
                frame_id: next_entry.last_jump_eid,
                sp: next_entry.sp,

                host_public_inputs,
                context_in_index,
                context_out_index,
                external_host_call_call_index,

                initial_memory_pages: next_entry.allocated_memory_pages,
                maximal_memory_pages: self.configure_table.maximal_memory_pages,

                #[cfg(feature = "continuation")]
                jops,
            }
        };

        post_initialization_state
    }
}
