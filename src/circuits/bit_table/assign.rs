use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use num_bigint::BigUint;
use specs::itable::UnaryOp;
use specs::step::StepInfo;

use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::Context;

use super::encode_bit_table_binary;
use super::encode_bit_table_popcnt;
use super::BitTableChip;
use super::BitTableOp;
use super::STEP_SIZE;

struct BitTableAssign {
    op: BitTableOp,
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
                ..
            } => Some(BitTableAssign {
                op: BitTableOp::Popcnt,
                left: *operand,
                right: 0,
                result: u64::from_le_bytes((*operand).to_le_bytes().map(|v| v.count_ones() as u8)),
            }),

            _ => None,
        })
        .collect::<Vec<_>>()
}

impl<F: FieldExt> BitTableChip<F> {
    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for _ in 0..self.max_available_rows / STEP_SIZE {
            ctx.region.assign_fixed(
                || "bit table: step sel",
                self.config.step_sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            for o in 0..4 {
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
        op: BitTableOp,
        left: u64,
        right: u64,
        result: u64,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "bit table encode",
            self.config.values[0],
            ctx.offset,
            || match op {
                BitTableOp::BinaryBit(op) => Ok(bn_to_field(&encode_bit_table_binary(
                    BigUint::from(op as u64),
                    BigUint::from(left),
                    BigUint::from(right),
                    BigUint::from(result),
                ))),
                BitTableOp::Popcnt => {
                    let result = result
                        .to_le_bytes()
                        .into_iter()
                        .fold(0u64, |acc, v| acc + v as u64);

                    Ok(bn_to_field(&encode_bit_table_popcnt(
                        BigUint::from(left),
                        BigUint::from(result),
                    )))
                }
            },
        )?;

        Ok(())
    }

    fn assign_unary_selector(&self, ctx: &mut Context<F>, op: BitTableOp) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "bit table encode",
            self.config.values[1],
            ctx.offset,
            || match op {
                BitTableOp::BinaryBit(_) => Ok(F::zero()),
                BitTableOp::Popcnt => Ok(F::one()),
            },
        )?;

        Ok(())
    }

    fn assign_op(&self, ctx: &mut Context<'_, F>, op: BitTableOp) -> Result<(), Error> {
        for i in 0..4 {
            ctx.region.assign_advice(
                || "bit table op",
                self.config.values[0],
                ctx.offset + 4 * i + 1,
                || Ok(F::from(op.index() as u64)),
            )?;

            ctx.region.assign_advice(
                || "bit table op",
                self.config.values[1],
                ctx.offset + 4 * i + 1,
                || Ok(F::from(op.index() as u64)),
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

        for (index, byte) in bytes.chunks_exact(2).enumerate() {
            ctx.region.assign_advice(
                || "bit table value",
                self.config.values[0],
                ctx.offset + 1 + 4 * index + offset,
                || Ok(F::from(byte[0] as u64)),
            )?;

            ctx.region.assign_advice(
                || "bit table value",
                self.config.values[1],
                ctx.offset + 1 + 4 * index + offset,
                || Ok(F::from(byte[1] as u64)),
            )?;
        }

        Ok(())
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        entries: Vec<BitTableAssign>,
    ) -> Result<(), Error> {
        assert!(entries.len() <= self.max_available_rows / STEP_SIZE);

        for entry in entries {
            self.assign_encode(ctx, entry.op, entry.left, entry.right, entry.result)?;
            self.assign_unary_selector(ctx, entry.op)?;
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
