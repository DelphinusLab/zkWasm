use super::Context;
use crate::{constant_from, curr};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct BitColumn<F: FieldExt> {
    pub col: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> BitColumn<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Self {
        let col = cols.next().unwrap();

        meta.create_gate("bit column", |meta| {
            vec![
                curr!(meta, col.clone())
                    * (constant_from!(1) - curr!(meta, col.clone()))
                    * enable(meta),
            ]
        });

        Self {
            col,
            _mark: PhantomData,
        }
    }

    pub fn assign(&self, ctx: &mut Context<F>, value: bool) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "bit value",
            self.col,
            ctx.offset,
            || Ok(if value { F::one() } else { F::zero() }),
        )?;

        Ok(())
    }
}
