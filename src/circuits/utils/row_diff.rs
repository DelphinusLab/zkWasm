use super::Context;
use crate::{constant_from, curr, next};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct RowDiffConfig<F: FieldExt> {
    pub data: Column<Advice>,
    pub same: Column<Advice>,
    pub inv: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> RowDiffConfig<F> {
    pub fn configure(
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Self {
        let data = cols.next().unwrap();
        let same = cols.next().unwrap();
        let inv = cols.next().unwrap();

        meta.enable_equality(same);

        meta.create_gate(key, |meta| {
            let enable = enable(meta);
            vec![
                (next!(meta, data) - curr!(meta, data)) * next!(meta, inv) + next!(meta, same)
                    - constant_from!(1),
                (next!(meta, data) - curr!(meta, data)) * next!(meta, same),
            ]
            .into_iter()
            .map(|x| x * enable.clone())
            .collect::<Vec<Expression<F>>>()
        });

        RowDiffConfig {
            data,
            same,
            inv,
            _mark: PhantomData,
        }
    }

    pub fn is_next_same(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        next!(meta, self.same)
    }

    pub fn is_same(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.same)
    }

    pub fn data(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.data)
    }

    pub fn diff_to_next(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        next!(meta, self.data) - curr!(meta, self.data)
    }

    pub fn assign(&self, ctx: &mut Context<F>, data: F, diff: F) -> Result<(), Error> {
        ctx.region
            .assign_advice(|| "row diff data", self.data, ctx.offset, || Ok(data))?;
        ctx.region.assign_advice(
            || "row diff inv",
            self.inv,
            ctx.offset,
            || Ok(diff.invert().unwrap_or(F::zero())),
        )?;

        if ctx.offset == 0 {
            ctx.region.assign_advice_from_constant(
                || "row diff same",
                self.same,
                ctx.offset,
                F::zero(),
            )?;
        } else {
            ctx.region.assign_advice(
                || "row diff same",
                self.same,
                ctx.offset,
                || {
                    Ok(if diff.is_zero().into() {
                        F::one()
                    } else {
                        F::zero()
                    })
                },
            )?;
        }

        Ok(())
    }

    pub fn assign_final(&self, ctx: &mut Context<F>) -> Result<(), Error> {
        ctx.region
            .assign_advice(|| "row diff data", self.data, ctx.offset, || Ok(F::zero()))?;
        ctx.region
            .assign_advice(|| "row diff inv", self.inv, ctx.offset, || Ok(F::zero()))?;
        ctx.region
            .assign_advice(|| "row diff same", self.same, ctx.offset, || Ok(F::zero()))?;

        Ok(())
    }
}
