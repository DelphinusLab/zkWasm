use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::jtable::StaticFrameEntry;
use specs::mtable::LocationType;
use specs::state::InitializationState;
use specs::CompilationTable;
use std::marker::PhantomData;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use crate::circuits::config::zkwasm_k;
use crate::circuits::utils::bn_to_field;
use crate::curr;

use super::test_circuit::RESERVE_ROWS;

mod assign;
mod configure;

pub const IMAGE_COL_NAME: &str = "img_col";
pub const INIT_MEMORY_ENTRIES_OFFSET: usize = 40960;
/*
 * 8192: 64 * 1024 / 8
 * A page is 64KB, an entry is 8B
 */
pub const PAGE_ENTRIES: u32 = 8192;

/// Compute maximal number of pages supported by the circuit.
/// circuit size - reserved rows for blind - initialization_state/static frame entries/instructions/br_table
///   - stack entries - global entries
pub fn compute_maximal_pages(k: u32) -> u32 {
    let bytes: u32 =
        ((1usize << k) - RESERVE_ROWS - INIT_MEMORY_ENTRIES_OFFSET - DEFAULT_VALUE_STACK_LIMIT * 2)
            .try_into()
            .unwrap();

    let pages = bytes / PAGE_ENTRIES;

    pages
}

pub(crate) struct InitMemoryLayouter {
    pub(crate) stack: u32,
    pub(crate) global: u32,
    pub(crate) pages: u32,
}

impl InitMemoryLayouter {
    fn for_each(self, mut f: impl FnMut((LocationType, u32))) {
        for offset in 0..self.stack {
            f((LocationType::Stack, offset))
        }

        for offset in 0..self.global {
            f((LocationType::Global, offset))
        }

        for offset in 0..(self.pages * PAGE_ENTRIES) {
            f((LocationType::Heap, offset))
        }
    }
}

pub struct ImageTableLayouter<T: Clone> {
    pub initialization_state: InitializationState<T>,
    pub static_frame_entries: Vec<(T, T)>,
    pub instructions: Option<Vec<T>>,
    pub br_table: Option<Vec<T>>,
    pub init_memory_entries: Option<Vec<T>>,
    pub rest_memory_writing_ops: Option<T>,
}

impl<F: FieldExt> ImageTableLayouter<F> {
    pub fn plain(&self) -> Vec<F> {
        let mut buf = vec![];

        buf.append(&mut self.initialization_state.plain());
        buf.append(
            &mut self
                .static_frame_entries
                .clone()
                .to_vec()
                .into_iter()
                .map(|(enable, fid)| vec![enable, fid])
                .collect::<Vec<Vec<_>>>()
                .concat(),
        );
        buf.append(&mut self.instructions.clone().unwrap());
        buf.append(&mut self.br_table.clone().unwrap());
        buf.append(&mut vec![F::zero(); INIT_MEMORY_ENTRIES_OFFSET - buf.len()]);
        buf.append(&mut self.init_memory_entries.clone().unwrap());

        buf
    }
}

pub trait EncodeCompilationTableValues<F: Clone> {
    fn encode_compilation_table_values(&self) -> ImageTableLayouter<F>;
}

impl<F: FieldExt> EncodeCompilationTableValues<F> for CompilationTable {
    fn encode_compilation_table_values(&self) -> ImageTableLayouter<F> {
        fn msg_of_initialization_state<F: FieldExt>(
            initialization_state: &InitializationState<u32>,
        ) -> InitializationState<F> {
            initialization_state.map(|field| F::from(*field as u64))
        }

        fn msg_of_instruction_table<F: FieldExt>(instruction_table: &InstructionTable) -> Vec<F> {
            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::Instruction.encode(BigUint::from(0u64)),
            ));

            for e in instruction_table.entries() {
                cells.push(bn_to_field(
                    &ImageTableEncoder::Instruction.encode(e.encode()),
                ));
            }

            cells
        }

        fn msg_of_br_table<F: FieldExt>(br_table: &BrTable, elem_table: &ElemTable) -> Vec<F> {
            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::BrTable.encode(BigUint::from(0u64)),
            ));

            for e in br_table.entries() {
                cells.push(bn_to_field(&ImageTableEncoder::BrTable.encode(e.encode())));
            }

            for e in elem_table.entries() {
                cells.push(bn_to_field(&ImageTableEncoder::BrTable.encode(e.encode())));
            }

            cells
        }

        fn msg_of_init_memory_table<F: FieldExt>(init_memory_table: &InitMemoryTable) -> Vec<F> {
            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
            ));

            let layouter = InitMemoryLayouter {
                stack: DEFAULT_VALUE_STACK_LIMIT as u32,
                global: DEFAULT_VALUE_STACK_LIMIT as u32,
                pages: compute_maximal_pages(zkwasm_k()),
            };

            layouter.for_each(|(ltype, offset)| {
                if let Some(entry) = init_memory_table.try_find(ltype, offset) {
                    cells.push(bn_to_field::<F>(
                        &ImageTableEncoder::InitMemory.encode(entry.encode()),
                    ));
                } else {
                    cells.push(bn_to_field::<F>(
                        &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
                    ));
                }
            });

            cells
        }

        fn msg_of_static_frame_table<F: FieldExt>(
            static_frame_table: &Vec<StaticFrameEntry>,
        ) -> Vec<(F, F)> {
            let mut cells = static_frame_table
                .into_iter()
                .map(|entry| (F::one(), bn_to_field(&entry.encode())))
                .collect::<Vec<_>>();

            cells.resize(
                2,
                (
                    F::zero(),
                    bn_to_field(
                        &StaticFrameEntry {
                            enable: false,
                            frame_id: 0,
                            next_frame_id: 0,
                            callee_fid: 0,
                            fid: 0,
                            iid: 0,
                        }
                        .encode(),
                    ),
                ),
            );

            cells
        }

        let initialization_state = msg_of_initialization_state(&self.initialization_state);
        let static_frame_entries = msg_of_static_frame_table(&self.static_jtable);

        let instructions = Some(msg_of_instruction_table(&self.itable));
        let br_table = Some(msg_of_br_table(
            &self.itable.create_brtable(),
            &self.elem_table,
        ));
        let init_memory_entries = Some(msg_of_init_memory_table(&self.imtable));

        ImageTableLayouter {
            initialization_state,
            static_frame_entries,
            instructions,
            br_table,
            init_memory_entries,
            rest_memory_writing_ops: None,
        }
    }
}

#[derive(Clone)]
pub struct ImageTableConfig<F: FieldExt> {
    _memory_addr_sel: Column<Fixed>,
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> ImageTableConfig<F> {
    pub fn expr(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.col)
    }
}

#[derive(Clone)]
pub struct ImageTableChip<F: FieldExt> {
    config: ImageTableConfig<F>,
}

impl<F: FieldExt> ImageTableChip<F> {
    pub fn new(config: ImageTableConfig<F>) -> Self {
        ImageTableChip { config }
    }
}
