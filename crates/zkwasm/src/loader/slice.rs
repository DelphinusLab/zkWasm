use halo2_proofs::arithmetic::FieldExt;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::configure_table::ConfigureTable;
use specs::etable::EventTable;
use specs::etable::EventTableBackend;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::JumpTable;
use specs::jtable::StaticFrameEntry;
use specs::jtable::STATIC_FRAME_ENTRY_NUMBER;
use specs::slice::Slice;
use specs::state::InitializationState;
use specs::Tables;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::circuits::ZkWasmCircuit;
use crate::runtime::state::UpdateInitMemoryTable;
use crate::runtime::state::UpdateInitializationState;

pub struct Slices<F: FieldExt> {
    itable: Arc<InstructionTable>,
    br_table: Arc<BrTable>,
    elem_table: Arc<ElemTable>,
    configure_table: Arc<ConfigureTable>,
    static_jtable: Arc<[StaticFrameEntry; STATIC_FRAME_ENTRY_NUMBER]>,
    frame_table: Arc<JumpTable>,

    imtable: Arc<InitMemoryTable>,
    initialization_state: Arc<InitializationState<u32>>,
    etables: VecDeque<EventTableBackend>,

    // the length of etable entries
    capability: usize,

    _marker: std::marker::PhantomData<F>,
}

impl<F: FieldExt> Slices<F> {
    pub fn new(tables: Tables, capability: usize) -> Self {
        Self {
            itable: tables.compilation_tables.itable,
            br_table: tables.compilation_tables.br_table,
            elem_table: tables.compilation_tables.elem_table,
            configure_table: tables.compilation_tables.configure_table,
            static_jtable: tables.compilation_tables.static_jtable,
            frame_table: Arc::new(tables.execution_tables.jtable),

            imtable: tables.compilation_tables.imtable,
            initialization_state: tables.compilation_tables.initialization_state,

            etables: tables.execution_tables.etable.into(),

            capability,

            _marker: std::marker::PhantomData,
        }
    }

    pub fn mock_test_all(self, k: u32, instances: Vec<F>) -> anyhow::Result<()> {
        use halo2_proofs::dev::MockProver;

        let mut iter = self.into_iter();

        while let Some(slice) = iter.next() {
            let prover = MockProver::run(k, &slice, vec![instances.clone()])?;
            assert_eq!(prover.verify(), Ok(()));
        }

        Ok(())
    }
}

impl<F: FieldExt> Iterator for Slices<F> {
    type Item = ZkWasmCircuit<F>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.etables.is_empty() {
            return None;
        }

        let etable = match self.etables.pop_front().unwrap() {
            EventTableBackend::Memory(etable) => etable,
            EventTableBackend::Json(path) => EventTable::from_json(&path).unwrap(),
        };

        let post_imtable = Arc::new(self.imtable.update_init_memory_table(&etable));
        let post_initialization_state = Arc::new({
            let next_event_entry = if let Some(next_event_table) = self.etables.front() {
                match next_event_table {
                    EventTableBackend::Memory(etable) => etable.entries().first().cloned(),
                    EventTableBackend::Json(path) => {
                        let etable = EventTable::from_json(&path).unwrap();
                        etable.entries().first().cloned()
                    }
                }
            } else {
                None
            };

            self.initialization_state.update_initialization_state(
                &etable,
                &self.configure_table,
                next_event_entry.as_ref(),
            )
        });

        let slice = Slice {
            itable: self.itable.clone(),
            br_table: self.br_table.clone(),
            elem_table: self.elem_table.clone(),
            configure_table: self.configure_table.clone(),
            static_jtable: self.static_jtable.clone(),
            frame_table: self.frame_table.clone(),

            imtable: self.imtable.clone(),
            post_imtable: post_imtable.clone(),

            initialization_state: self.initialization_state.clone(),
            post_initialization_state: post_initialization_state.clone(),

            etable: Arc::new(etable),
            is_last_slice: self.etables.is_empty(),
        };

        self.imtable = post_imtable;
        self.initialization_state = post_initialization_state;

        Some(ZkWasmCircuit::new(slice, self.capability))
    }
}
