use super::Context;
use crate::{constant_from, curr, nextn};
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
    pub distance: u32,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> RowDiffConfig<F> {
    pub fn configure(
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        distance: u32,
        enable: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Self {
        let data = cols.next().unwrap();
        let same = cols.next().unwrap();
        let inv = cols.next().unwrap();

        meta.create_gate(key, |meta| {
            let enable = enable(meta);
            let diff = nextn!(meta, data, distance as i32) - curr!(meta, data);
            let inv = nextn!(meta, inv, distance as i32);
            let same = nextn!(meta, same, distance as i32);
            vec![
                diff.clone() * inv.clone() - same.clone() - constant_from!(1),
                diff * same,
            ]
            .into_iter()
            .map(|x| x * enable.clone())
            .collect::<Vec<Expression<F>>>()
        });

        RowDiffConfig {
            data,
            same,
            inv,
            distance,
            _mark: PhantomData,
        }
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

        if ctx.offset < self.distance as usize {
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
}
