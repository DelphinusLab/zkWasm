use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;

use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::image_table::ImageTableLayouter;
use crate::circuits::mtable::MemoryTableConfig;

use super::PostImageTableChipTrait;
use super::PostImageTableConfigTrait;

#[derive(Clone)]
pub(in crate::circuits) struct TrivialPostImageTableConfig<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableConfigTrait<F> for TrivialPostImageTableConfig<F> {
    fn configure(
        _meta: &mut ConstraintSystem<F>,
        _memory_addr_sel: Column<Fixed>,
        _memory_table: &MemoryTableConfig<F>,
        _pre_image_table: &ImageTableConfig<F>,
    ) -> Self {
        Self { _mark: PhantomData }
    }
}

pub(in crate::circuits) struct TrivialPostImageTableChip<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableChipTrait<F, TrivialPostImageTableConfig<F>>
    for TrivialPostImageTableChip<F>
{
    fn new(_config: TrivialPostImageTableConfig<F>, _circuit_maximal_pages: u32) -> Self {
        Self { _mark: PhantomData }
    }

    fn assign(
        self,
        _layouter: &mut impl Layouter<F>,
        _pre_image_table: ImageTableLayouter<F>,
        _post_image_table: ImageTableLayouter<F>,
        _permutation_cells: ImageTableLayouter<AssignedCell<F, F>>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
