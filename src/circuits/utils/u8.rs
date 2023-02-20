use super::Context;
use crate::{circuits::rtable::RangeTableConfig, curr};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct U8Column<F: FieldExt> {
    pub col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> U8Column<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Self {
        let col = cols.next().unwrap();

        rtable.configure_in_u8_range(meta, "u8", |meta| curr!(meta, col) * enable(meta));

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
