use halo2_proofs::arithmetic::FieldExt;
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::Tables;

use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;
use crate::runtime::state::UpdateCompilationTable;

pub struct Slice {
    table: Tables,
    capability: usize,
}

impl Slice {
    pub fn build_circuit<F: FieldExt>(self) -> TestCircuit<F> {
        println!(
            "etable entries: {}",
            self.table.execution_tables.etable.entries().len()
        );

        let builder = ZkWasmCircuitBuilder { tables: self.table };

        builder.build_circuit(Some(self.capability))
    }
}

pub struct Slices {
    remaining_etable_entries: Vec<EventTableEntry>,
    last_compilation_table: CompilationTable,

    // the length of etable entries
    capability: usize,

    origin_table: Tables,
}

impl Slices {
    pub fn new(tables: Tables, capability: usize) -> Self {
        Self {
            remaining_etable_entries: tables.execution_tables.etable.entries().clone(),
            last_compilation_table: tables.compilation_tables.clone(),
            capability,
            origin_table: tables,
        }
    }

    fn pop_etable_entries(&mut self) -> Vec<EventTableEntry> {
        self.remaining_etable_entries
            .drain(0..self.capability.min(self.remaining_etable_entries.len()))
            .collect::<Vec<_>>()
    }
}

impl Iterator for Slices {
    type Item = Slice;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_etable_entries.is_empty() {
            return None;
        }

        let mut etable_entries = self.pop_etable_entries();

        let (updated_init_memory_table, updated_post_initialization_state) = {
            let updated_init_memory_table = self
                .last_compilation_table
                .update_init_memory_table(&etable_entries);

            let is_last_slice = self.remaining_etable_entries.is_empty();

            // If it is not the last slice, push a helper step to get the post initialization state.
            if !is_last_slice {
                etable_entries.push(self.remaining_etable_entries.first().unwrap().clone());
            }

            let updated_post_initialization_state = self
                .last_compilation_table
                .update_initialization_state(&etable_entries, is_last_slice);

            if !is_last_slice {
                etable_entries.pop();
            }

            (updated_init_memory_table, updated_post_initialization_state)
        };

        let execution_tables = ExecutionTable {
            etable: EventTable::new(etable_entries),
            jtable: self.origin_table.execution_tables.jtable.clone(),
        };

        let post_image_table = CompilationTable {
            itable: self.origin_table.compilation_tables.itable.clone(),
            imtable: updated_init_memory_table,
            br_table: self.origin_table.compilation_tables.br_table.clone(),
            elem_table: self.origin_table.compilation_tables.elem_table.clone(),
            configure_table: self.origin_table.compilation_tables.configure_table.clone(),
            static_jtable: self.origin_table.compilation_tables.static_jtable.clone(),
            initialization_state: updated_post_initialization_state,
        };

        let compilation_tables =
            std::mem::replace(&mut self.last_compilation_table, post_image_table.clone());

        let slice = Slice {
            table: Tables {
                compilation_tables,
                execution_tables,
                post_image_table,
                is_last_slice: self.remaining_etable_entries.is_empty(),
            },
            capability: self.capability,
        };

        Some(slice)
    }
}
