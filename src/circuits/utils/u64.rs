use super::Context;
use crate::{circuits::rtable::RangeTableConfig, constant, curr};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use std::marker::PhantomData;

/* U64Config can guarantee that value is in range of u64 */
#[derive(Clone)]
pub struct U64Config<F: FieldExt> {
    pub u16_le: [Column<Advice>; 4],
    pub value: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> U64Config<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Self {
        let u16_le = [0; 4].map(|_| cols.next().unwrap());
        let value = cols.next().unwrap();

        for u16_i in u16_le.iter() {
            rtable.configure_in_u16_range(meta, "u16", |meta| {
                curr!(meta, u16_i.clone()) * enable(meta)
            });
        }

        meta.create_gate("u64 sum", |meta| {
            let mut acc = curr!(meta, u16_le[0].clone());
            let mut base = F::one();
            for i in 1..4usize {
                base = base * F::from(1 << 16 as u64);
                acc = acc + constant!(base) * curr!(meta, u16_le[i].clone());
            }
            vec![(acc - curr!(meta, value.clone())) * enable(meta)]
        });

        Self {
            u16_le,
            value,
            _mark: PhantomData,
        }
    }

    pub fn assign(&self, ctx: &mut Context<F>, value: u64) -> Result<(), Error> {
        ctx.region.as_ref().borrow_mut().assign_advice(
            || "u64 value",
            self.value.clone(),
            ctx.offset,
            || Ok(value.into()),
        )?;

        let mut bytes = Vec::from(value.to_le_bytes());
        bytes.resize(8, 0);

        for i in 0..4 {
            ctx.region.as_ref().borrow_mut().assign_advice(
                || "u64 byte",
                self.u16_le[i],
                ctx.offset,
                || Ok((((bytes[i * 2 + 1] as u64) << 8) + bytes[i * 2] as u64).into()),
            )?;
        }

        Ok(())
    }
}
