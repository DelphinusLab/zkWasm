use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use std::marker::PhantomData;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use crate::curr;

use super::test_circuit::RESERVE_ROWS;
use super::utils::image_table::INIT_MEMORY_ENTRIES_OFFSET;

mod assign;
mod configure;

pub const IMAGE_COL_NAME: &str = "img_col";
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
