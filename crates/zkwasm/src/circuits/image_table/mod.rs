use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::TableColumn;
use num_bigint::BigUint;
use specs::brtable::BrTable;
use specs::brtable::ElemTable;
use specs::encode::image_table::ImageTableEncoder;
use specs::imtable::InitMemoryTable;
use specs::itable::InstructionTable;
use specs::mtable::LocationType;
use specs::CompilationTable;
use std::marker::PhantomData;

use crate::circuits::config::max_image_table_rows;
use crate::circuits::utils::bn_to_field;

mod assign;
mod configure;

pub const IMAGE_COL_NAME: &str = "img_col";

pub struct ImageTableLayouter<T: Clone> {
    /*
     * include:
     *   instruction table
     *   br table
     *   elem table
     *   init memory table
     */
    pub lookup_entries: Option<Vec<T>>,
}

impl<T: Clone> ImageTableLayouter<T> {
    pub fn plain(&self) -> Vec<T> {
        self.lookup_entries.clone().unwrap()
    }
}

pub trait EncodeCompilationTableValues<F: Clone> {
    fn encode_compilation_table_values(&self) -> ImageTableLayouter<F>;
}

impl<F: FieldExt> EncodeCompilationTableValues<F> for CompilationTable {
    fn encode_compilation_table_values(&self) -> ImageTableLayouter<F> {
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
            let heap_entries = init_memory_table.filter(LocationType::Heap);
            let global_entries = init_memory_table.filter(LocationType::Global);

            let mut cells = vec![];

            cells.push(bn_to_field(
                &ImageTableEncoder::InitMemory.encode(BigUint::from(0u64)),
            ));

            for v in heap_entries.into_iter().chain(global_entries.into_iter()) {
                cells.push(bn_to_field::<F>(
                    &ImageTableEncoder::InitMemory.encode(v.encode()),
                ));
            }

            cells
        }

        fn msg_of_image_table<F: FieldExt>(
            instruction_table: &InstructionTable,
            br_table: &BrTable,
            elem_table: &ElemTable,
            init_memory_table: &InitMemoryTable,
        ) -> Vec<F> {
            let mut cells = vec![];

            cells.append(&mut msg_of_instruction_table(instruction_table));
            cells.append(&mut msg_of_br_table(br_table, elem_table));
            cells.append(&mut msg_of_init_memory_table(init_memory_table));

            for _ in cells.len()..(max_image_table_rows() as usize) {
                cells.push(F::zero());
            }

            cells
        }

        let lookup_entries = msg_of_image_table(
            &self.itable,
            &self.itable.create_brtable(),
            &self.elem_table,
            &self.imtable,
        );

        ImageTableLayouter {
            lookup_entries: Some(lookup_entries),
        }
    }
}

#[derive(Clone)]
pub struct ImageTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
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
