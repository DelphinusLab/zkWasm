use std::collections::BTreeMap;

use specs::etable::EventTable;
use specs::host_function::HostPlugin;
use specs::imtable::InitMemoryTable;
use specs::imtable::InitMemoryTableEntry;
use specs::itable::Opcode;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::state::InitializationState;
use specs::step::StepInfo;
use specs::CompilationTable;

use super::memory_event_of_step;

pub(crate) trait UpdateCompilationTable {
    fn update_init_memory_table(&self, execution_table: &EventTable) -> InitMemoryTable;

    fn update_initialization_state(
        &self,
        execution_table: &EventTable,
        is_last_slice: bool,
    ) -> InitializationState<u32>;
}

impl UpdateCompilationTable for CompilationTable {
    fn update_init_memory_table(&self, execution_table: &EventTable) -> InitMemoryTable {
        let mut local_map = BTreeMap::<u32, InitMemoryTableEntry>::new();
        let mut global_map = BTreeMap::<u32, InitMemoryTableEntry>::new();
        let mut memory_map = BTreeMap::<u32, InitMemoryTableEntry>::new();

        // First insert origin imtable entries which may be overwritten.
        for entry in self.imtable.entries() {
            match entry.ltype {
                LocationType::Stack => {
                    assert_eq!(entry.start_offset, entry.end_offset);

                    local_map.insert(entry.start_offset, entry.clone());
                }
                LocationType::Heap => {
                    for offset in entry.start_offset..=entry.end_offset {
                        memory_map.insert(
                            offset,
                            InitMemoryTableEntry {
                                ltype: entry.ltype,
                                is_mutable: entry.is_mutable,
                                start_offset: offset,
                                end_offset: offset,
                                vtype: entry.vtype,
                                value: entry.value,
                                eid: entry.eid,
                            },
                        );
                    }
                }
                LocationType::Global => {
                    assert_eq!(entry.start_offset, entry.end_offset);

                    global_map.insert(entry.start_offset, entry.clone());
                }
            }
        }

        for etable_entry in execution_table.entries() {
            let memory_writing_entires = memory_event_of_step(etable_entry)
                .into_iter()
                .filter(|entry| entry.atype == AccessType::Write);

            for mentry in memory_writing_entires {
                let map = match mentry.ltype {
                    LocationType::Stack => &mut local_map,
                    LocationType::Heap => &mut memory_map,
                    LocationType::Global => &mut global_map,
                };

                map.insert(
                    mentry.offset,
                    InitMemoryTableEntry {
                        ltype: mentry.ltype,
                        is_mutable: mentry.is_mutable,
                        start_offset: mentry.offset,
                        end_offset: mentry.offset,
                        vtype: mentry.vtype,
                        value: mentry.value,
                        eid: etable_entry.eid,
                    },
                );
            }
        }

        let mut init_memory_entries = vec![];
        init_memory_entries.append(&mut local_map.into_iter().map(|(_, v)| v).collect::<Vec<_>>());
        init_memory_entries.append(&mut global_map.into_iter().map(|(_, v)| v).collect::<Vec<_>>());
        init_memory_entries.append(&mut memory_map.into_iter().map(|(_, v)| v).collect::<Vec<_>>());

        InitMemoryTable::new(init_memory_entries)
    }

    fn update_initialization_state(
        &self,
        execution_table: &EventTable,
        is_last_slice: bool,
    ) -> InitializationState<u32> {
        let mut host_public_inputs = self.initialization_state.host_public_inputs;
        let mut context_in_index = self.initialization_state.context_in_index;
        let mut context_out_index = self.initialization_state.context_out_index;
        let mut external_host_call_call_index =
            self.initialization_state.external_host_call_call_index;

        #[cfg(feature = "continuation")]
        let mut jops = self.initialization_state.jops;

        for entry in execution_table.entries() {
            match &entry.step_info {
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
                StepInfo::Call { .. } | StepInfo::Return { .. } => {
                    #[cfg(feature = "continuation")]
                    {
                        jops += 1;
                    }
                }
                _ => (),
            }
        }

        let last_entry = execution_table.entries().last().unwrap();

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
                // TODO: why not constant 4095?
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
