use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use std::marker::PhantomData;

use crate::curr;

use super::test_circuit::RESERVE_ROWS;
use super::utils::image_table::GLOBAL_CAPABILITY;
use super::utils::image_table::INIT_MEMORY_ENTRIES_OFFSET;
use super::utils::image_table::STACK_CAPABILITY;

mod assign;
mod configure;

pub const IMAGE_COL_NAME: &str = "img_col";

pub const PAGE_SIZE: u32 = 64 * 1024;
// A block is 8 bytes
pub const PAGE_ENTRIES: u32 = PAGE_SIZE / 8;

/// Compute maximal number of pages supported by the circuit.
/// circuit size - reserved rows for blind - init memory entries base offset
///   - stack entries - global entries
pub fn compute_maximal_pages(k: u32) -> u32 {
    let rows: u32 = ((1usize << k)
        - RESERVE_ROWS
        - INIT_MEMORY_ENTRIES_OFFSET
        - STACK_CAPABILITY
        - GLOBAL_CAPABILITY)
        .try_into()
        .unwrap();

    // A block is 8 bytes.
    let bytes = rows * 8;

    bytes / PAGE_SIZE
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ImageTableConfig<F: FieldExt> {
    memory_addr_sel: Option<Column<Fixed>>,
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
