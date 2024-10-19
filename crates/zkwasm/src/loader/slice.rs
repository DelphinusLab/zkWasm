use halo2_proofs::arithmetic::FieldExt;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::configure_table::ConfigureTable;
use specs::etable::EventTable;
use specs::external_host_call_table::ExternalHostCallTable;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::CalledFrameTable;
use specs::jtable::InheritedFrameTable;
use specs::slice::FrameTableSlice;
use specs::slice::Slice;
use specs::slice_backend::SliceBackend;
use specs::state::InitializationState;
use specs::Tables;
use std::collections::VecDeque;
use std::iter::Peekable;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::circuits::ZkWasmCircuit;
use crate::error::BuildingCircuitError;
use crate::runtime::state::UpdateInitMemoryTable;
use crate::runtime::state::UpdateInitializationState;

pub struct Slices<F: FieldExt, B: SliceBackend> {
    k: u32,

    // The number of trivial circuits left.
    padding: usize,

    itable: Arc<InstructionTable>,
    br_table: Arc<BrTable>,
    elem_table: Arc<ElemTable>,
    configure_table: Arc<ConfigureTable>,
    initial_frame_table: Arc<InheritedFrameTable>,

    imtable: Arc<InitMemoryTable>,
    initialization_state: Arc<InitializationState<u32>>,

    slices: Vec<B>,
    context_input_table: Arc<Vec<u64>>,
    context_output_table: Arc<Vec<u64>>,

    _marker: std::marker::PhantomData<F>,
}

impl<F: FieldExt, B: SliceBackend> Slices<F, B> {
    /*
     * padding: Insert trivial slices so that the number of proofs is at least padding.
     */
    pub fn new(
        k: u32,
        tables: Tables<B>,
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

        for slice in self.into_iter() {
            match slice {
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

pub struct SlicesWrap<B: SliceBackend>(Vec<B>);
pub struct SlicesIter<B: SliceBackend>(VecDeque<B>);

impl<B: SliceBackend> IntoIterator for SlicesWrap<B> {
    type Item = specs::slice_backend::Slice;

    type IntoIter = SlicesIter<B>;

    fn into_iter(self) -> Self::IntoIter {
        SlicesIter(self.0.into())
    }
}

impl<B: SliceBackend> Iterator for SlicesIter<B> {
    type Item = specs::slice_backend::Slice;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front().map(|slice| slice.into())
    }
}

pub struct ZkWasmCircuitIter<F: FieldExt, B: SliceBackend> {
    // immutable parts
    k: u32,
    itable: Arc<InstructionTable>,
    br_table: Arc<BrTable>,
    elem_table: Arc<ElemTable>,
    configure_table: Arc<ConfigureTable>,
    initial_frame_table: Arc<InheritedFrameTable>,
    context_input_table: Arc<Vec<u64>>,
    context_output_table: Arc<Vec<u64>>,

    // mutable parts
    // The number of trivial circuits left.
    padding: usize,
    imtable: Arc<InitMemoryTable>,
    initialization_state: Arc<InitializationState<u32>>,
    slices: Peekable<SlicesIter<B>>,

    mark: PhantomData<F>,
}

impl<F: FieldExt, B: SliceBackend> IntoIterator for Slices<F, B> {
    type Item = ZkWasmCircuit<F>;

    type IntoIter = ZkWasmCircuitIter<F, B>;

    fn into_iter(self) -> Self::IntoIter {
        ZkWasmCircuitIter {
            k: self.k,
            itable: self.itable,
            br_table: self.br_table,
            elem_table: self.elem_table,
            configure_table: self.configure_table,
            initial_frame_table: self.initial_frame_table,
            context_input_table: self.context_input_table,
            context_output_table: self.context_output_table,
            padding: self.padding,
            imtable: self.imtable,
            initialization_state: self.initialization_state,
            slices: SlicesWrap(self.slices).into_iter().peekable(),
            mark: PhantomData,
        }
    }
}

impl<F: FieldExt, B: SliceBackend> ZkWasmCircuitIter<F, B> {
    // create a circuit slice with all entries disabled.
    fn trivial_slice(&mut self) -> ZkWasmCircuit<F> {
        self.padding -= 1;

        let frame_table = Arc::new(FrameTableSlice {
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

        ZkWasmCircuit::new(self.k, slice).unwrap()
    }
}

impl<F: FieldExt, B: SliceBackend> Iterator for ZkWasmCircuitIter<F, B> {
    type Item = ZkWasmCircuit<F>;

    fn next(&mut self) -> Option<Self::Item> {
        // return if it's last
        self.slices.peek()?;

        if self.padding > 0 {
            return Some(self.trivial_slice());
        }

        let slice = self.slices.next().unwrap();
        let frame_table = slice.frame_table.into();
        let external_host_call_table = slice.external_host_call_table;
        let etable = slice.etable;

        let post_imtable = Arc::new(self.imtable.update_init_memory_table(&etable));

        let post_initialization_state = {
            let next_first_eentry = self
                .slices
                .peek()
                .map(|slice| slice.etable.entries().first().cloned().unwrap());

            let post_initialization_state = self.initialization_state.update_initialization_state(
                &etable,
                &self.configure_table,
                next_first_eentry.as_ref(),
            );

            Arc::new(post_initialization_state)
        };

        let post_inherited_frame_table =
            self.slices
                .peek()
                .map_or_else(InheritedFrameTable::default, |next_slice| {
                    let post_inherited_frame_table = next_slice.frame_table.inherited.clone();

                    (*post_inherited_frame_table).clone().try_into().unwrap()
                });

        let slice = Slice {
            itable: self.itable.clone(),
            br_table: self.br_table.clone(),
            elem_table: self.elem_table.clone(),
            configure_table: self.configure_table.clone(),
            initial_frame_table: self.initial_frame_table.clone(),

            frame_table: Arc::new(frame_table),
            post_inherited_frame_table: Arc::new(post_inherited_frame_table),

            imtable: self.imtable.clone(),
            post_imtable: post_imtable.clone(),

            initialization_state: self.initialization_state.clone(),
            post_initialization_state: post_initialization_state.clone(),

            etable: Arc::new(etable),
            external_host_call_table: Arc::new(external_host_call_table),
            context_input_table: self.context_input_table.clone(),
            context_output_table: self.context_output_table.clone(),

            is_last_slice: self.slices.peek().is_none(),
        };

        self.imtable = post_imtable;
        self.initialization_state = post_initialization_state;

        let circuit = ZkWasmCircuit::new(self.k, slice).unwrap();

        Some(circuit)
    }
}
