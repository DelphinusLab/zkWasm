use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;

// image data: 8192
// frame table: 4
// event table: 1

#[derive(Clone)]
pub(crate) struct CheckSumConfig<F: FieldExt> {
    img_col: Column<Advice>,
    _mark: PhantomData<F>,
}

pub const IMAGE_COL_NAME: &str = "img_col";

pub(crate) struct CheckSumChip<F: FieldExt> {
    config: CheckSumConfig<F>,
}

impl<F: FieldExt> CheckSumConfig<F> {
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let img_col = meta.named_advice_column(IMAGE_COL_NAME.to_owned());

        Self {
            img_col,
            _mark: PhantomData,
        }
    }
}

impl<F: FieldExt> CheckSumChip<F> {
    pub(crate) fn new(config: CheckSumConfig<F>) -> Self {
        Self { config }
    }

    pub(crate) fn assign(
        &self,
        layouter: &mut impl Layouter<F>,
        img: Vec<AssignedCell<F, F>>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "image column",
            |mut region| {
                for (i, v) in img.iter().enumerate() {
                    region.assign_advice(
                        || "img data",
                        self.config.img_col,
                        i,
                        || Ok(v.value().unwrap().clone()),
                    )?;
                }

                Ok(())
            },
        )
    }
}
