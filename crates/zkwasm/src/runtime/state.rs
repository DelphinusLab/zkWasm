use specs::configure_table::ConfigureTable;
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::host_function::HostPlugin;
use specs::imtable::InitMemoryTable;
use specs::imtable::InitMemoryTableEntry;
use specs::mtable::AccessType;
use specs::state::InitializationState;
use specs::step::StepInfo;

use super::memory_event_of_step;

pub(crate) trait UpdateInitMemoryTable {
    fn update_init_memory_table(&self, execution_table: &EventTable) -> Self;
}

pub(crate) trait UpdateInitializationState {
    fn update_initialization_state(
        &self,
        execution_table: &EventTable,
        configure_table: &ConfigureTable,
        next_event_entry: Option<&EventTableEntry>,
    ) -> Self;
}

impl UpdateInitMemoryTable for InitMemoryTable {
    fn update_init_memory_table(&self, execution_table: &EventTable) -> InitMemoryTable {
        // First insert origin imtable entries which may be overwritten.
        let mut map = self.0.clone();

        let it = execution_table.entries().iter();
        for etable_entry in it {
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
}

impl UpdateInitializationState for InitializationState<u32> {
    fn update_initialization_state(
        &self,
        execution_table: &EventTable,
        configure_table: &ConfigureTable,
        // None indicates last slice
        next_event_entry: Option<&EventTableEntry>,
    ) -> InitializationState<u32> {
        let mut host_public_inputs = self.host_public_inputs;
        let mut context_in_index = self.context_in_index;
        let mut context_out_index = self.context_out_index;
        let mut external_host_call_call_index = self.external_host_call_call_index;

        for entry in execution_table.entries() {
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
                _ => (),
            }
        }

        let post_initialization_state = if let Some(next_entry) = next_event_entry {
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
                maximal_memory_pages: configure_table.maximal_memory_pages,
            }
        } else {
            let last_entry = execution_table.entries().last().unwrap();

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
                maximal_memory_pages: configure_table.maximal_memory_pages,
            }
        };

        post_initialization_state
    }
}
