use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;

use crate::circuits::image_table::ImageTableLayouter;

use super::PostImageTableChipTrait;
use super::PostImageTableConfigTrait;

#[derive(Clone)]
pub(in crate::circuits) struct TrivialPostImageTableConfig<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableConfigTrait<F> for TrivialPostImageTableConfig<F> {
    fn configure(_meta: &mut ConstraintSystem<F>) -> Self {
        Self { _mark: PhantomData }
    }
}

pub(in crate::circuits) struct TrivialPostImageTableChip<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> PostImageTableChipTrait<F, TrivialPostImageTableConfig<F>>
    for TrivialPostImageTableChip<F>
{
    fn new(_config: TrivialPostImageTableConfig<F>) -> Self {
        Self { _mark: PhantomData }
    }

    fn assign(
        self,
        _layouter: &mut impl Layouter<F>,
        _image_table: ImageTableLayouter<F>,
        _permutation_cells: ImageTableLayouter<Cell>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
