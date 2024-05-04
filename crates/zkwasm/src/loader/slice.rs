use halo2_proofs::arithmetic::FieldExt;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::configure_table::ConfigureTable;
use specs::etable::EventTable;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::FrameTable;
use specs::jtable::InheritedFrameTable;
use specs::slice::Slice;
use specs::state::InitializationState;
use specs::TableBackend;
use specs::Tables;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::circuits::ZkWasmCircuit;
use crate::error::BuildingCircuitError;
use crate::runtime::state::UpdateInitMemoryTable;
use crate::runtime::state::UpdateInitializationState;

pub struct Slices<F: FieldExt> {
    k: u32,

    itable: Arc<InstructionTable>,
    br_table: Arc<BrTable>,
    elem_table: Arc<ElemTable>,
    configure_table: Arc<ConfigureTable>,
    initial_frame_table: Arc<InheritedFrameTable>,
    frame_table: VecDeque<TableBackend<FrameTable>>,

    imtable: Arc<InitMemoryTable>,
    initialization_state: Arc<InitializationState<u32>>,
    etables: VecDeque<TableBackend<EventTable>>,

    _marker: std::marker::PhantomData<F>,
}

impl<F: FieldExt> Slices<F> {
    pub fn new(k: u32, tables: Tables) -> Result<Self, BuildingCircuitError> {
        if cfg!(not(feature = "continuation")) {
            let slices = tables.execution_tables.etable.len();

            if slices != 1 {
                return Err(BuildingCircuitError::MultiSlicesNotSupport(slices));
            }
        }

        Ok(Self {
            k,

            itable: tables.compilation_tables.itable,
            br_table: tables.compilation_tables.br_table,
            elem_table: tables.compilation_tables.elem_table,
            configure_table: tables.compilation_tables.configure_table,
            initial_frame_table: tables.compilation_tables.initial_frame_table,

            frame_table: tables.execution_tables.frame_table.into(),

            imtable: tables.compilation_tables.imtable,
            initialization_state: tables.compilation_tables.initialization_state,

            etables: tables.execution_tables.etable.into(),

            _marker: std::marker::PhantomData,
        })
    }

    pub fn mock_test_all(self, instances: Vec<F>) -> anyhow::Result<()> {
        use halo2_proofs::dev::MockProver;

        let k = self.k;
        let mut iter = self.into_iter();

        while let Some(slice) = iter.next() {
            match slice? {
                ZkWasmCircuit::Ongoing(circuit) => {
                    let prover = MockProver::run(k, &circuit, vec![instances.clone()])?;
                    assert_eq!(prover.verify(), Ok(()));
                }
                ZkWasmCircuit::LastSliceCircuit(circuit) => {
                    let prover = MockProver::run(k, &circuit, vec![instances.clone()])?;
                    assert_eq!(prover.verify(), Ok(()));
                }
            }
        }

        Ok(())
    }
}

impl<F: FieldExt> Iterator for Slices<F> {
    type Item = Result<ZkWasmCircuit<F>, BuildingCircuitError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.etables.is_empty() {
            return None;
        }

        let etable = match self.etables.pop_front().unwrap() {
            TableBackend::Memory(etable) => etable,
            TableBackend::Json(path) => EventTable::read(&path).unwrap(),
        };

        let post_imtable = Arc::new(self.imtable.update_init_memory_table(&etable));
        let post_initialization_state = Arc::new({
            let next_event_entry = if let Some(next_event_table) = self.etables.front() {
                match next_event_table {
                    TableBackend::Memory(etable) => etable.entries().first().cloned(),
                    TableBackend::Json(path) => {
                        let etable = EventTable::read(&path).unwrap();
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

        let frame_table = match self.frame_table.pop_front().unwrap() {
            TableBackend::Memory(frame_table) => frame_table,
            TableBackend::Json(path) => FrameTable::read(&path).unwrap(),
        }
        .into();

        let post_inherited_frame_table = self.frame_table.front().map_or(
            Arc::new(InheritedFrameTable::default()),
            |frame_table| {
                let post_inherited_frame_table = match frame_table {
                    TableBackend::Memory(frame_table) => frame_table.inherited.clone(),
                    TableBackend::Json(path) => FrameTable::read(&path).unwrap().inherited,
                };

                Arc::new((*post_inherited_frame_table).clone().try_into().unwrap())
            },
        );

        let slice = Slice {
            itable: self.itable.clone(),
            br_table: self.br_table.clone(),
            elem_table: self.elem_table.clone(),
            configure_table: self.configure_table.clone(),
            initial_frame_table: self.initial_frame_table.clone(),

            frame_table: Arc::new(frame_table),
            post_inherited_frame_table,

            imtable: self.imtable.clone(),
            post_imtable: post_imtable.clone(),

            initialization_state: self.initialization_state.clone(),
            post_initialization_state: post_initialization_state.clone(),

            etable: Arc::new(etable),
            is_last_slice: self.etables.is_empty(),
        };

        self.imtable = post_imtable;
        self.initialization_state = post_initialization_state;

        let circuit = ZkWasmCircuit::new(self.k, slice);

        Some(circuit)
    }
}
