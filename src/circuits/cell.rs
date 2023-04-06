use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;

use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::Context;
use crate::nextn;

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
    fn assign(&self, ctx: &mut Context<'_, F>, value: F) -> Result<AssignedCell<F, F>, Error>;
    fn assign_bn(
        &self,
        ctx: &mut Context<'_, F>,
        value: &BigUint,
    ) -> Result<AssignedCell<F, F>, Error> {
        self.assign(ctx, bn_to_field(value))
    }
    fn assign_u32(
        &self,
        ctx: &mut Context<'_, F>,
        value: u32,
    ) -> Result<AssignedCell<F, F>, Error> {
        self.assign(ctx, (value as u64).into())
    }
    fn assign_bool(
        &self,
        ctx: &mut Context<'_, F>,
        value: bool,
    ) -> Result<AssignedCell<F, F>, Error> {
        self.assign(ctx, if value { F::one() } else { F::zero() })
    }
}

impl<F: FieldExt> CellExpression<F> for AllocatedCell<F> {
    fn curr_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }

    fn assign(&self, ctx: &mut Context<'_, F>, value: F) -> Result<AssignedCell<F, F>, Error> {
        ctx.region.assign_advice(
            || "assign cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(value),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedU64Cell<F: FieldExt> {
    pub(crate) u16_cells_le: [AllocatedU16Cell<F>; 4],
    pub(crate) u64_cell: AllocatedUnlimitedCell<F>,
}

impl<F: FieldExt> AllocatedU64Cell<F> {
    pub(crate) fn expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.u64_cell.expr(meta)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedU64CellWithFlagBit<F: FieldExt, const POS: usize> {
    pub(crate) u16_cells_le: [AllocatedU16Cell<F>; 4],
    pub(crate) u64_cell: AllocatedUnlimitedCell<F>,
    pub(crate) flag_bit_cell: AllocatedBitCell<F>,
    pub(crate) flag_u16_rem_cell: AllocatedCommonRangeCell<F>,
    pub(crate) flag_u16_rem_diff_cell: AllocatedCommonRangeCell<F>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedU64CellWithFlagBitDyn<F: FieldExt> {
    pub(crate) u16_cells_le: [AllocatedU16Cell<F>; 4],
    pub(crate) u64_cell: AllocatedUnlimitedCell<F>,
    pub(crate) flag_bit_cell: AllocatedBitCell<F>,
    pub(crate) flag_u16_rem_cell: AllocatedCommonRangeCell<F>,
    pub(crate) flag_u16_rem_diff_cell: AllocatedCommonRangeCell<F>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedU64CellWithFlagBitDynSign<F: FieldExt> {
    pub(crate) u16_cells_le: [AllocatedU16Cell<F>; 4],
    pub(crate) u64_cell: AllocatedUnlimitedCell<F>,
    pub(crate) flag_bit_cell: AllocatedBitCell<F>,
    pub(crate) flag_u16_rem_cell: AllocatedCommonRangeCell<F>,
    pub(crate) flag_u16_rem_diff_cell: AllocatedCommonRangeCell<F>,
}

macro_rules! define_cell {
    ($x: ident) => {
        #[derive(Debug, Clone, Copy)]
        pub(crate) struct $x<F: FieldExt>(pub(crate) AllocatedCell<F>);

        impl<F: FieldExt> CellExpression<F> for $x<F> {
            fn curr_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
                self.0.curr_expr(meta)
            }

            fn assign(
                &self,
                ctx: &mut Context<'_, F>,
                value: F,
            ) -> Result<AssignedCell<F, F>, Error> {
                self.0.assign(ctx, value)
            }
        }
    };
}

define_cell!(AllocatedBitCell);
define_cell!(AllocatedCommonRangeCell);
define_cell!(AllocatedU8Cell);
define_cell!(AllocatedU16Cell);
define_cell!(AllocatedUnlimitedCell);

impl<F: FieldExt> AllocatedU64Cell<F> {
    pub(crate) fn assign(&self, ctx: &mut Context<'_, F>, value: u64) -> Result<(), Error> {
        for i in 0..4 {
            self.u16_cells_le[i].assign(ctx, ((value >> (i * 16)) & 0xffffu64).into())?;
        }
        self.u64_cell.assign(ctx, value.into())?;
        Ok(())
    }
}

impl<F: FieldExt, const POS: usize> AllocatedU64CellWithFlagBit<F, POS> {
    pub(crate) fn assign(&self, ctx: &mut Context<'_, F>, value: u64) -> Result<(), Error> {
        for i in 0..4 {
            self.u16_cells_le[i].assign(ctx, ((value >> (i * 16)) & 0xffffu64).into())?;
        }
        self.u64_cell.assign(ctx, value.into())?;

        let u16_value = (value >> (POS * 16)) & 0xffff;
        let u16_flag_bit = u16_value >> 15;
        let u16_rem = u16_value & 0x7fff;
        let u16_rem_diff = 0x7fff - u16_rem;
        self.flag_bit_cell.assign_u32(ctx, u16_flag_bit as u32)?;
        self.flag_u16_rem_cell.assign_u32(ctx, u16_rem as u32)?;
        self.flag_u16_rem_diff_cell
            .assign_u32(ctx, u16_rem_diff as u32)?;
        Ok(())
    }
}

impl<F: FieldExt> AllocatedU64CellWithFlagBitDyn<F> {
    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        value: u64,
        is_i32: bool,
    ) -> Result<(), Error> {
        for i in 0..4 {
            self.u16_cells_le[i].assign(ctx, ((value >> (i * 16)) & 0xffffu64).into())?;
        }
        self.u64_cell.assign(ctx, value.into())?;

        let pos = if is_i32 { 1 } else { 3 };
        let u16_value = (value >> (pos * 16)) & 0xffff;
        let u16_flag_bit = u16_value >> 15;
        let u16_rem = u16_value & 0x7fff;
        let u16_rem_diff = 0x7fff - u16_rem;
        self.flag_bit_cell.assign_u32(ctx, u16_flag_bit as u32)?;
        self.flag_u16_rem_cell.assign_u32(ctx, u16_rem as u32)?;
        self.flag_u16_rem_diff_cell
            .assign_u32(ctx, u16_rem_diff as u32)?;
        Ok(())
    }
}

impl<F: FieldExt> AllocatedU64CellWithFlagBitDynSign<F> {
    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        value: u64,
        is_i32: bool,
        is_sign: bool,
    ) -> Result<(), Error> {
        for i in 0..4 {
            self.u16_cells_le[i].assign(ctx, ((value >> (i * 16)) & 0xffffu64).into())?;
        }
        self.u64_cell.assign(ctx, value.into())?;

        if is_sign {
            let pos = if is_i32 { 1 } else { 3 };
            let u16_value = (value >> (pos * 16)) & 0xffff;
            let u16_flag_bit = u16_value >> 15;
            let u16_rem = u16_value & 0x7fff;
            let u16_rem_diff = 0x7fff - u16_rem;
            self.flag_bit_cell.assign_u32(ctx, u16_flag_bit as u32)?;
            self.flag_u16_rem_cell.assign_u32(ctx, u16_rem as u32)?;
            self.flag_u16_rem_diff_cell
                .assign_u32(ctx, u16_rem_diff as u32)?;
        }
        Ok(())
    }
}
