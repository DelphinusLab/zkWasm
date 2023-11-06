use halo2_proofs::arithmetic::FieldExt;
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::mtable::MemoryWritingTable;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::InitializationState;
use specs::Tables;

use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;
use crate::runtime::memory_event_of_step;
use crate::runtime::state::simulate_execution;

pub struct Slice {
    table: Tables,
    capability: usize,
}

impl Slice {
    pub fn build_circuit<F: FieldExt>(self) -> TestCircuit<F> {
        let builder = ZkWasmCircuitBuilder { tables: self.table };

        builder.build_slice_circuit(self.capability)
    }
}

pub struct Slices {
    // States for Iterator
    remaining_etable_entries: Vec<EventTableEntry>,
    current_compilation_table: CompilationTable,

    // the length of etable entries
    capability: usize,

    origin_table: Tables,
    // used to build post imtable
    full_memory_writing_table: MemoryWritingTable,
}

impl Slices {
    pub fn pop_etable_entries(&mut self) -> Vec<EventTableEntry> {
        self.remaining_etable_entries
            .drain(0..self.capability.min(self.remaining_etable_entries.len()))
            .collect::<Vec<_>>()
    }

    pub fn new(table: Tables, capability: usize) -> Self {
        let full_memory_writing_table = table
            .execution_tables
            .etable
            .create_memory_table(&table.compilation_tables.imtable, memory_event_of_step)
            .into();
        let remaining_etable_entries = table.execution_tables.etable.entries().clone();

        Self {
            remaining_etable_entries,
            capability,
            current_compilation_table: table.compilation_tables.clone(),
            origin_table: table,
            full_memory_writing_table,
        }
    }
}

impl Iterator for Slices {
    type Item = Slice;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_etable_entries.is_empty() {
            return None;
        }

        let execution_tables = ExecutionTable {
            etable: EventTable::new(self.pop_etable_entries()),
            jtable: self.origin_table.execution_tables.jtable.clone(),
        };

        let is_last_slice = self.remaining_etable_entries.is_empty();

        if !is_last_slice {
            self.remaining_etable_entries
                .insert(0, execution_tables.etable.entries().last().unwrap().clone());
        }

        let post_state = simulate_execution(
            &self.current_compilation_table,
            &execution_tables,
            &self.full_memory_writing_table,
            memory_event_of_step,
        );

        let post_image_table = CompilationTable {
            itable: self.origin_table.compilation_tables.itable.clone(),
            imtable: post_state.0,
            elem_table: self.origin_table.compilation_tables.elem_table.clone(),
            configure_table: self.origin_table.compilation_tables.configure_table.clone(),
            static_jtable: self.origin_table.compilation_tables.static_jtable.clone(),
            initialization_state: if is_last_slice {
                post_state.1
            } else {
                let next_state = self.remaining_etable_entries.first().unwrap();

                InitializationState {
                    eid: next_state.eid,
                    fid: next_state.inst.fid,
                    iid: next_state.inst.iid,
                    frame_id: next_state.last_jump_eid,
                    sp: next_state.sp,
                    host_public_inputs: post_state.1.host_public_inputs,
                    context_in_index: post_state.1.context_in_index,
                    context_out_index: post_state.1.context_out_index,
                    external_host_call_call_index: post_state.1.external_host_call_call_index,
                    initial_memory_pages: next_state.allocated_memory_pages,
                    maximal_memory_pages: self
                        .origin_table
                        .compilation_tables
                        .configure_table
                        .maximal_memory_pages,
                    jops: post_state.1.jops,
                }
            },
        };

        let slice = Slice {
            table: Tables {
                compilation_tables: self.current_compilation_table.clone(),
                execution_tables,
                post_image_table: post_image_table.clone(),
                is_last_slice: self.remaining_etable_entries.is_empty(),
            },
            capability: self.capability,
        };

        self.current_compilation_table = post_image_table;

        Some(slice)
    }
}
