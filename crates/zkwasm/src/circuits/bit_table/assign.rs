use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Error;
use specs::itable::UnaryOp;
use specs::step::StepInfo;

use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::Context;

use super::BitTableChip;
use super::BitTableOp;
use super::BitTableTrait;
use super::BLOCK_SEL_OFFSET;
use super::STEP_SIZE;
use super::U32_OFFSET;
use super::U8_OFFSET;

pub(crate) struct BitTableAssign {
    op: BitTableOp,
    left: u64,
    right: u64,
    result: u64,
}

impl BitTableTrait for EventTableWithMemoryInfo {
    fn filter_bit_table_entries(&self) -> Vec<BitTableAssign> {
        self.0
            .iter()
            .filter_map(|entry| match &entry.eentry.step_info {
                StepInfo::I32BinBitOp {
                    class,
                    left,
                    right,
                    value,
                } => Some(BitTableAssign {
                    op: BitTableOp::BinaryBit(*class),
                    left: *left as u32 as u64,
                    right: *right as u32 as u64,
                    result: *value as u32 as u64,
                }),

                StepInfo::I64BinBitOp {
                    class,
                    left,
                    right,
                    value,
                } => Some(BitTableAssign {
                    op: BitTableOp::BinaryBit(*class),
                    left: *left as u64,
                    right: *right as u64,
                    result: *value as u64,
                }),

                StepInfo::UnaryOp {
                    class: UnaryOp::Popcnt,
                    operand,
                    result,
                    ..
                } => Some(BitTableAssign {
                    op: BitTableOp::Popcnt,
                    left: *operand,
                    right: 0,
                    result: *result,
                }),

                _ => None,
            })
            .collect::<Vec<_>>()
    }
}

impl<F: FieldExt> BitTableChip<F> {
    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for _ in 0..self.max_available_rows / STEP_SIZE {
            ctx.region.assign_fixed(
                || "bit table: block sel",
                self.config.block_sel,
                ctx.offset + BLOCK_SEL_OFFSET,
                || Ok(F::one()),
            )?;

            for i in U8_OFFSET {
                ctx.region.assign_fixed(
                    || "bit table: lookup sel",
                    self.config.lookup_sel,
                    ctx.offset + i,
                    || Ok(F::one()),
                )?;
            }

            for i in U32_OFFSET {
                ctx.region.assign_fixed(
                    || "bit table: u32 sel",
                    self.config.u32_sel,
                    ctx.offset + i,
                    || Ok(F::one()),
                )?;
            }

            ctx.step(STEP_SIZE);
        }

        Ok(())
    }

    fn assign_op(&self, ctx: &mut Context<'_, F>, op: BitTableOp) -> Result<(), Error> {
        let op_index = F::from(op.index() as u64);

        for i in 0..STEP_SIZE {
            ctx.region.assign_advice(
                || "bit table op",
                self.config.op,
                ctx.offset + i,
                || Ok(op_index),
            )?;
        }

        if op == BitTableOp::Popcnt {
            for i in U32_OFFSET {
                ctx.region.assign_advice(
                    || "bit table op",
                    self.config.helper,
                    ctx.offset + i,
                    || Ok(F::one()),
                )?;
            }
        }

        Ok(())
    }

    fn assign_u64_popcnt(
        &self,
        ctx: &mut Context<'_, F>,
        col: Column<Advice>,
        value: u64,
    ) -> Result<(), Error> {
        let low_u32 = value as u32;
        let high_u32 = (value >> 32) as u32;

        let low_u8s_popcnt = low_u32.to_le_bytes().map(|byte| byte.count_ones());
        let high_u8s_popcnt = high_u32.to_le_bytes().map(|byte| byte.count_ones());
        let low_u32_popcnt = low_u32.count_ones();
        let high_u32_popcnt = high_u32.count_ones();

        ctx.region.assign_advice(
            || "bit table: assign u64",
            col,
            ctx.offset,
            || Ok(F::from(value.count_ones() as u64)),
        )?;

        macro_rules! assign_u32 {
            ($v: expr, $bytes: expr, $offset: expr) => {{
                ctx.region.assign_advice(
                    || "bit table: assign u32",
                    col,
                    ctx.offset + $offset,
                    || Ok(F::from($v as u64)),
                )?;

                for (index, byte_count_ones) in $bytes.into_iter().enumerate() {
                    ctx.region.assign_advice(
                        || "bit table: assign u8",
                        col,
                        ctx.offset + 1 + index + $offset,
                        || Ok(F::from(byte_count_ones as u64)),
                    )?;
                }
            }};
        }

        assign_u32!(low_u32_popcnt, low_u8s_popcnt, U32_OFFSET[0]);
        assign_u32!(high_u32_popcnt, high_u8s_popcnt, U32_OFFSET[1]);

        Ok(())
    }

    fn assign_u64_le(
        &self,
        ctx: &mut Context<'_, F>,
        col: Column<Advice>,
        value: u64,
    ) -> Result<(), Error> {
        let low_u32 = value as u32;
        let high_u32 = (value >> 32) as u32;

        ctx.region.assign_advice(
            || "bit table: assign u64",
            col,
            ctx.offset,
            || Ok(F::from(value)),
        )?;

        macro_rules! assign_u32 {
            ($v: expr, $offset: expr) => {{
                let bytes = $v.to_le_bytes();

                ctx.region.assign_advice(
                    || "bit table: assign u32",
                    col,
                    ctx.offset + $offset,
                    || Ok(F::from($v as u64)),
                )?;

                for (index, byte) in bytes.into_iter().enumerate() {
                    ctx.region.assign_advice(
                        || "bit table: assign u8",
                        col,
                        ctx.offset + 1 + index + $offset,
                        || Ok(F::from(byte as u64)),
                    )?;
                }
            }};
        }

        assign_u32!(low_u32, U32_OFFSET[0]);
        assign_u32!(high_u32, U32_OFFSET[1]);

        Ok(())
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        entries: &Vec<BitTableAssign>,
    ) -> Result<(), Error> {
        assert!(entries.len() <= self.max_available_rows / STEP_SIZE);

        for entry in entries {
            self.assign_op(ctx, entry.op)?;
            self.assign_u64_le(ctx, self.config.left, entry.left)?;
            self.assign_u64_le(ctx, self.config.right, entry.right)?;
            if entry.op == BitTableOp::Popcnt {
                // entry.left: Compute left's decompose popcnt
                self.assign_u64_popcnt(ctx, self.config.result, entry.left)?;
            } else {
                self.assign_u64_le(ctx, self.config.result, entry.result)?;
            }

            ctx.step(STEP_SIZE);
        }

        Ok(())
    }

    pub(crate) fn assign(
        &self,
        layouter: impl Layouter<F>,
        event_table: Vec<BitTableAssign>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "bit table",
            |region| {
                let mut ctx = Context::new(region);

                self.init(&mut ctx)?;

                ctx.reset();

                self.assign_entries(&mut ctx, &event_table)?;

                Ok(())
            },
        )
    }
}
