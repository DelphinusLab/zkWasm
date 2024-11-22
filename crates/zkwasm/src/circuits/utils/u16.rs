use crate::circuits::rtable::RangeTableConfig;
use crate::curr;

use super::Context;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct U16Column<F: FieldExt> {
    pub col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> U16Column<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) -> Self {
        let col = meta.advice_column();

        rtable.configure_in_u16_range(meta, "u16", |meta| curr!(meta, col));

        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn assign(&self, ctx: &mut Context<F>, value: u64) -> Result<(), Error> {
        ctx.region
            .assign_advice(|| "u16 value", self.col, ctx.offset, || Ok(value.into()))?;

        Ok(())
    }
}
