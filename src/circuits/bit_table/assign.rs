use halo2_proofs::{arithmetic::FieldExt, plonk::Error};
use num_bigint::BigUint;
use specs::{itable::BitOp, step::StepInfo};

use crate::circuits::utils::{bn_to_field, table_entry::EventTableWithMemoryInfo, Context};

use super::{encode_bit_table, BitTableChip, STEP_SIZE};

struct BitTableAssign {
    op: BitOp,
    left: u64,
    right: u64,
    result: u64,
}

fn filter_bit_table_entries(event_table: &EventTableWithMemoryInfo) -> Vec<BitTableAssign> {
    event_table
        .0
        .iter()
        .filter_map(|entry| match &entry.eentry.step_info {
            StepInfo::I32BinBitOp {
                class,
                left,
                right,
                value,
            } => Some(BitTableAssign {
                op: *class,
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
                op: *class,
                left: *left as u64,
                right: *right as u64,
                result: *value as u64,
            }),
            _ => None,
        })
        .collect::<Vec<_>>()
}

impl<F: FieldExt> BitTableChip<F> {
    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for i in 0..self.max_available_rows / STEP_SIZE {
            ctx.region.assign_fixed(
                || "bit table: step sel",
                self.config.step_sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            for o in 0..8 {
                ctx.region.assign_fixed(
                    || "bit table: lookup sel",
                    self.config.lookup_sel,
                    ctx.offset + 4 * o + 1,
                    || Ok(F::one()),
                )?;
            }

            ctx.step(STEP_SIZE);
        }

        Ok(())
    }

    fn assign_encode(
        &self,
        ctx: &mut Context<F>,
        op: BitOp,
        left: u64,
        right: u64,
        result: u64,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "bit table encode",
            self.config.value,
            ctx.offset,
            || {
                Ok(bn_to_field(&encode_bit_table(
                    BigUint::from(op as u64),
                    BigUint::from(left),
                    BigUint::from(right),
                    BigUint::from(result),
                )))
            },
        )?;

        Ok(())
    }

    fn assign_op(&self, ctx: &mut Context<'_, F>, op: BitOp) -> Result<(), Error> {
        for i in 0..8 {
            ctx.region.assign_advice(
                || "bit table op",
                self.config.value,
                ctx.offset + 4 * i + 1,
                || Ok(F::from(op as u64)),
            )?;
        }

        Ok(())
    }

    fn assign_u64_le(
        &self,
        ctx: &mut Context<'_, F>,
        offset: usize,
        value: u64,
    ) -> Result<(), Error> {
        let mut bytes = value.to_le_bytes().to_vec();
        bytes.resize(8, 0);

        for (index, byte) in bytes.iter().enumerate() {
            ctx.region.assign_advice(
                || "bit table value",
                self.config.value,
                ctx.offset + 1 + 4 * index + offset,
                || Ok(F::from(*byte as u64)),
            )?;
        }

        Ok(())
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        entries: Vec<BitTableAssign>,
    ) -> Result<(), Error> {
        for entry in entries {
            self.assign_encode(ctx, entry.op, entry.left, entry.right, entry.result)?;
            self.assign_op(ctx, entry.op)?;
            self.assign_u64_le(ctx, 1, entry.left)?;
            self.assign_u64_le(ctx, 2, entry.right)?;
            self.assign_u64_le(ctx, 3, entry.result)?;

            ctx.step(STEP_SIZE);
        }

        Ok(())
    }

    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        event_table: &EventTableWithMemoryInfo,
    ) -> Result<(), Error> {
        self.init(ctx)?;

        ctx.reset();

        self.assign_entries(ctx, filter_bit_table_entries(event_table))?;

        Ok(())
    }
}
