use super::Context;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct U8Column<F: FieldExt> {
    pub col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> U8Column<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        (l_0, l_active, l_active_last): (Column<Fixed>, Column<Fixed>, Column<Fixed>),
    ) -> Self {
        let col = meta.advice_column_range(
            l_0,
            l_active,
            l_active_last,
            (0, F::from(0)),
            (u8::MAX as u32, F::from(u8::MAX as u64)),
            (1, F::one()),
        );

        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn assign(&self, ctx: &mut Context<F>, value: u64) -> Result<(), Error> {
        ctx.region
            .assign_advice(|| "u8 value", self.col, ctx.offset, || Ok(value.into()))?;

        Ok(())
    }
}
