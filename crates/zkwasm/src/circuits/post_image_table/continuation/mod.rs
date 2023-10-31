use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Error;

use crate::circuits::image_table::ImageTableChip;
use crate::circuits::image_table::ImageTableConfig;
use crate::circuits::image_table::ImageTableLayouter;

use super::PostImageTableChipTrait;
use super::PostImageTableConfigTrait;

#[derive(Clone)]
pub(in crate::circuits) struct ContinuationPostImageTableConfig<F: FieldExt> {
    config: ImageTableConfig<F>,
}

impl<F: FieldExt> PostImageTableConfigTrait<F> for ContinuationPostImageTableConfig<F> {
    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self {
        Self {
            config: ImageTableConfig::configure(meta),
        }
    }
}

pub(in crate::circuits) struct ContinuationPostImageTableChip<F: FieldExt> {
    chip: ImageTableChip<F>,
}

impl<F: FieldExt> PostImageTableChipTrait<F, ContinuationPostImageTableConfig<F>>
    for ContinuationPostImageTableChip<F>
{
    fn new(config: ContinuationPostImageTableConfig<F>) -> Self {
        Self {
            chip: ImageTableChip::new(config.config),
        }
    }

    fn assign(
        self,
        layouter: &mut impl Layouter<F>,
        image_table: ImageTableLayouter<F>,
        permutation_cells: ImageTableLayouter<Cell>,
    ) -> Result<(), Error> {
        self.chip.assign(layouter, image_table, permutation_cells)
    }
}
