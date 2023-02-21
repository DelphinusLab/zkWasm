use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, Error, Expression, VirtualCells},
};
use num_bigint::BigUint;

use crate::{
    circuits::utils::{bn_to_field, Context},
    nextn,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedCell<F: FieldExt> {
    pub(crate) col: Column<Advice>,
    pub(crate) rot: i32,
    pub(crate) _mark: PhantomData<F>,
}

pub(crate) trait CellExpression<F: FieldExt> {
    fn curr_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.curr_expr(meta)
    }
    fn assign(&self, ctx: &mut Context<'_, F>, value: F) -> Result<(), Error>;
    fn assign_bn(&self, ctx: &mut Context<'_, F>, value: &BigUint) -> Result<(), Error> {
        self.assign(ctx, bn_to_field(value))
    }
}

impl<F: FieldExt> CellExpression<F> for AllocatedCell<F> {
    fn curr_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }

    fn assign(&self, ctx: &mut Context<'_, F>, value: F) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "assign cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(value),
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedU64Cell<F: FieldExt> {
    pub(crate) u16_cells_le: [AllocatedU16Cell<F>; 4],
    pub(crate) u64_cell: AllocatedUnlimitedCell<F>,
}

macro_rules! define_cell {
    ($x: ident) => {
        #[derive(Debug, Clone, Copy)]
        pub(crate) struct $x<F: FieldExt>(pub(crate) AllocatedCell<F>);

        impl<F: FieldExt> CellExpression<F> for $x<F> {
            fn curr_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
                self.0.curr_expr(meta)
            }

            fn assign(&self, ctx: &mut Context<'_, F>, value: F) -> Result<(), Error> {
                self.0.assign(ctx, value)
            }
        }
    };
}

define_cell!(AllocatedBitCell);
define_cell!(AllocatedCommonRangeCell);
define_cell!(AllocatedU16Cell);
define_cell!(AllocatedUnlimitedCell);
define_cell!(AllocatedMemoryTableLookupCell);

impl<F: FieldExt> AllocatedU64Cell<F> {
    pub(crate) fn assign(&self, ctx: &mut Context<'_, F>, value: u64) -> Result<(), Error> {
        for i in 0..4 {
            self.u16_cells_le[i].assign(ctx, ((value >> (i * 16)) & 0xffffu64).into())?;
        }
        self.u64_cell.assign(ctx, value.into())?;
        Ok(())
    }
}
