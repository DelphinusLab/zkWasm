use super::{bytes8::Bytes8Config, Context};
use crate::{circuits::rtable::RangeTableConfig, constant_from, curr};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::mtable::VarType;

#[derive(Clone)]
pub struct TValueConfig<F: FieldExt> {
    pub vtype: Column<Advice>,
    pub value: Bytes8Config<F>,
}

impl<F: FieldExt> TValueConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Self {
        let value = Bytes8Config::configure(meta, cols, rtable, &enable);
        let vtype = cols.next().unwrap();

        for i in 0..8usize {
            rtable.configure_in_vtype_byte_range(
                meta,
                "tvalue byte",
                |meta| {
                    (
                        constant_from!(i),
                        curr!(meta, vtype.clone()),
                        curr!(meta, value.bytes_le[i].clone()),
                    )
                },
                &enable,
            );
        }

        Self { vtype, value }
    }

    pub fn assign(&self, ctx: &mut Context<F>, vtype: VarType, value: u64) -> Result<(), Error> {
        self.value.assign(ctx, value)?;

        ctx.region.assign_advice(
            || "tvalue vtype",
            self.vtype.clone(),
            ctx.offset,
            || Ok((vtype as u64).into()),
        )?;

        Ok(())
    }
}
