use std::collections::BTreeMap;

use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::etable::EventTableWithMemoryInfo;
use specs::host_function::HostPlugin;
use specs::imtable::InitMemoryTable;
use specs::imtable::InitMemoryTableEntry;
use specs::itable::Opcode;
use specs::mtable::LocationType;
use specs::mtable::MemoryTableEntry;
use specs::mtable::MemoryWritingTable;
use specs::step::StepInfo;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::InitializationState;

pub fn update(
    raw_init_memory_table: &InitMemoryTable,
    execution_table: &EventTable,
    // must be consistent for all slices
    full_memory_writing_table: &MemoryWritingTable,
    memory_event_of_step: fn(&EventTableEntry, &mut u32) -> Vec<MemoryTableEntry>,
) -> InitMemoryTable {
    let mut local_map = BTreeMap::<u32, InitMemoryTableEntry>::new();
    let mut global_map = BTreeMap::<u32, InitMemoryTableEntry>::new();
    let mut memory_map = BTreeMap::<u32, InitMemoryTableEntry>::new();

    // First insert imtable entries which may be overwritten.
    for entry in raw_init_memory_table.entries() {
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

                            start_eid: entry.start_eid,
                            end_eid: entry.end_eid,
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

    let event_table = EventTableWithMemoryInfo::new(
        execution_table,
        full_memory_writing_table,
        memory_event_of_step,
    );

    for etable_entry in event_table.0 {
        for mentry in etable_entry.memory_rw_entires {
            let map = match mentry.entry.ltype {
                LocationType::Stack => &mut local_map,
                LocationType::Heap => &mut memory_map,
                LocationType::Global => &mut global_map,
            };

            map.insert(
                mentry.entry.offset,
                InitMemoryTableEntry {
                    ltype: mentry.entry.ltype,
                    is_mutable: mentry.entry.is_mutable,
                    start_offset: mentry.entry.offset,
                    end_offset: mentry.entry.offset,
                    vtype: mentry.entry.vtype,
                    value: mentry.entry.value,
                    start_eid: mentry.start_eid,
                    end_eid: mentry.end_eid,
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

pub fn simulate_execution(
    compilation_table: &CompilationTable,
    execution_table: &ExecutionTable,
    full_memory_writing_table: &MemoryWritingTable,
    memory_event_of_step: fn(&EventTableEntry, &mut u32) -> Vec<MemoryTableEntry>,
) -> (InitMemoryTable, InitializationState<u32>) {
    let mut host_public_inputs = compilation_table.initialization_state.host_public_inputs;
    let mut context_in_index = compilation_table.initialization_state.context_in_index;
    let mut context_out_index = compilation_table.initialization_state.context_out_index;
    let mut external_host_call_call_index = compilation_table
        .initialization_state
        .external_host_call_call_index;

    #[cfg(feature = "continuation")]
    let mut jops = compilation_table.initialization_state.jops;

    for entry in execution_table.etable.entries() {
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

    let last_entry = execution_table.etable.entries().last().unwrap();

    let post_initialization_state = InitializationState {
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
        maximal_memory_pages: compilation_table.configure_table.maximal_memory_pages,

        #[cfg(feature = "continuation")]
        jops,
    };

    (
        update(
            &compilation_table.imtable,
            &execution_table.etable,
            full_memory_writing_table,
            memory_event_of_step,
        ),
        post_initialization_state,
    )
}
