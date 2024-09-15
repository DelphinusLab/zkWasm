use halo2_proofs::arithmetic::FieldExt;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::configure_table::ConfigureTable;
use specs::etable::EventTable;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::host_function::ContextInputTable;
use specs::host_function::ContextOutputTable;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::CalledFrameTable;
use specs::jtable::FrameTable;
use specs::jtable::InheritedFrameTable;
use specs::slice::Slice;
use specs::slice_backend::SliceBackend;
use specs::state::InitializationState;
use specs::Tables;
use std::sync::Arc;

use crate::circuits::ZkWasmCircuit;
use crate::error::BuildingCircuitError;
use crate::runtime::state::UpdateInitMemoryTable;
use crate::runtime::state::UpdateInitializationState;

pub struct Slices<F: FieldExt> {
    k: u32,

    // The number of trivial circuits left.
    padding: usize,

    itable: Arc<InstructionTable>,
    br_table: Arc<BrTable>,
    elem_table: Arc<ElemTable>,
    configure_table: Arc<ConfigureTable>,
    initial_frame_table: InheritedFrameTable,

    imtable: Arc<InitMemoryTable>,
    initialization_state: Arc<InitializationState<u32>>,

    slices: Box<dyn SliceBackend>,
    context_input_table: Arc<ContextInputTable>,
    context_output_table: Arc<ContextOutputTable>,

    _marker: std::marker::PhantomData<F>,
}

impl<F: FieldExt> Slices<F> {
    /*
     * padding: Insert trivial slices so that the number of proofs is at least padding.
     */
    pub fn new(
        k: u32,
        tables: Tables,
        padding: Option<usize>,
    ) -> Result<Self, BuildingCircuitError> {
        let slices_len = tables.execution_tables.slice_backend.len();

        if cfg!(not(feature = "continuation")) && slices_len != 1 {
            return Err(BuildingCircuitError::MultiSlicesNotSupport(slices_len));
        }

        let padding = padding.map_or(0, |padding| padding.saturating_sub(slices_len));

        Ok(Self {
            k,

            padding,

            itable: tables.compilation_tables.itable,
            br_table: tables.compilation_tables.br_table,
            elem_table: tables.compilation_tables.elem_table,
            configure_table: tables.compilation_tables.configure_table,
            initial_frame_table: tables.compilation_tables.initial_frame_table,
            imtable: tables.compilation_tables.imtable,
            initialization_state: tables.compilation_tables.initialization_state,

            slices: tables.execution_tables.slice_backend,
            context_input_table: tables.execution_tables.context_input_table.into(),
            context_output_table: tables.execution_tables.context_output_table.into(),

            _marker: std::marker::PhantomData,
        })
    }

    pub fn mock_test_all(self, instances: Vec<F>) -> anyhow::Result<()> {
        use halo2_proofs::dev::MockProver;

        let k = self.k;
        let iter = self;

        for slice in iter {
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

impl<F: FieldExt> Slices<F> {
    // create a circuit slice with all entries disabled.
    fn trivial_slice(&mut self) -> Result<ZkWasmCircuit<F>, BuildingCircuitError> {
        self.padding -= 1;

        let frame_table = Arc::new(FrameTable {
            inherited: self.initial_frame_table.clone(),
            called: CalledFrameTable::default(),
        });

        let slice = Slice {
            itable: self.itable.clone(),
            br_table: self.br_table.clone(),
            elem_table: self.elem_table.clone(),
            configure_table: self.configure_table.clone(),
            initial_frame_table: self.initial_frame_table.clone(),

            frame_table,
            post_inherited_frame_table: self.initial_frame_table.clone(),

            imtable: self.imtable.clone(),
            post_imtable: self.imtable.clone(),

            initialization_state: self.initialization_state.clone(),
            post_initialization_state: self.initialization_state.clone(),

            etable: Arc::new(EventTable::default()),
            external_host_call_table: Arc::new(ExternalHostCallTable::default()),
            context_input_table: self.context_input_table.clone(),
            context_output_table: self.context_output_table.clone(),

            is_last_slice: false,
        };

        ZkWasmCircuit::new(self.k, slice)
    }
}

impl<F: FieldExt> Iterator for Slices<F> {
    type Item = Result<ZkWasmCircuit<F>, BuildingCircuitError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.slices.is_empty() {
            return None;
        }

        if self.padding > 0 {
            return Some(self.trivial_slice());
        }

        let slice = self.slices.pop().unwrap();
        let external_host_call_table = slice.external_host_call_table;
        let etable = slice.etable;

        let post_imtable = Arc::new(self.imtable.update_init_memory_table(&etable));
        let post_initialization_state = Arc::new({
            let next_event_entry = if let Some(slice) = self.slices.first() {
                slice.etable.entries().first().cloned()
            } else {
                None
            };

            self.initialization_state.update_initialization_state(
                &etable,
                &self.configure_table,
                next_event_entry.as_ref(),
            )
        });

        let post_inherited_frame_table = self
            .slices
            .first()
            .map_or_else(InheritedFrameTable::default, |slice| {
                slice.frame_table.inherited.clone()
            });

        let slice = Slice {
            itable: self.itable.clone(),
            br_table: self.br_table.clone(),
            elem_table: self.elem_table.clone(),
            configure_table: self.configure_table.clone(),
            initial_frame_table: self.initial_frame_table.clone(),

            frame_table: slice.frame_table.into(),
            post_inherited_frame_table,

            imtable: self.imtable.clone(),
            post_imtable: post_imtable.clone(),

            initialization_state: self.initialization_state.clone(),
            post_initialization_state: post_initialization_state.clone(),

            etable: Arc::new(etable),
            external_host_call_table: Arc::new(external_host_call_table),
            context_input_table: self.context_input_table.clone(),
            context_output_table: self.context_output_table.clone(),

            is_last_slice: self.slices.is_empty(),
        };

        self.imtable = post_imtable;
        self.initialization_state = post_initialization_state;

        let circuit = ZkWasmCircuit::new(self.k, slice);

        Some(circuit)
    }
}
