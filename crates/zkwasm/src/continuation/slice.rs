use halo2_proofs::arithmetic::FieldExt;
use specs::etable::EventTable;
use specs::etable::EventTableEntry;
use specs::CompilationTable;
use specs::ExecutionTable;
use specs::Tables;

use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;
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
}

impl Slices {
    pub fn pop_etable_entries(&mut self) -> Vec<EventTableEntry> {
        let entries = self
            .remaining_etable_entries
            .drain(0..self.capability.min(self.remaining_etable_entries.len()))
            .collect::<Vec<_>>();

        if !self.remaining_etable_entries.is_empty() {
            self.remaining_etable_entries
                .insert(0, entries.last().unwrap().clone());
        };

        entries
    }

    pub fn new(table: Tables, capability: usize) -> Self {
        let remaining_etable_entries = table.execution_tables.etable.entries().clone();

        Self {
            remaining_etable_entries,
            capability,
            current_compilation_table: table.compilation_tables.clone(),
            origin_table: table,
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

        let post_image_table =
            simulate_execution(&self.current_compilation_table, &execution_tables);

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
