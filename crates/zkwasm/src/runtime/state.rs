use specs::host_function::HostPlugin;
use specs::itable::Opcode;
use specs::step::StepInfo;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::InitializationState;

pub fn simulate_execution(
    compilation_table: &CompilationTable,
    execution_table: &ExecutionTable,
) -> CompilationTable {
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
                unreachable!()
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

    CompilationTable {
        itable: compilation_table.itable.clone(),
        // FIXME: update imtable
        imtable: compilation_table.imtable.clone(),
        elem_table: compilation_table.elem_table.clone(),
        configure_table: compilation_table.configure_table,
        static_jtable: compilation_table.static_jtable.clone(),
        initialization_state: post_initialization_state,
    }
}
