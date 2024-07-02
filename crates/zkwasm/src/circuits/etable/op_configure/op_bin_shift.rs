use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::rtable::pow_table_power_encode;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use crate::constant_from_bn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use specs::itable::ShiftOp;
use specs::itable::OPCODE_ARG0_SHIFT;
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::step::StepInfo;

pub struct BinShiftConfig<F: FieldExt> {
    lhs: AllocatedU64CellWithFlagBitDyn<F>,
    rhs: AllocatedU64Cell<F>,
    round: AllocatedU64Cell<F>,
    rem: AllocatedU64Cell<F>,
    diff: AllocatedU64Cell<F>,
    pad: AllocatedUnlimitedCell<F>,
    res: AllocatedUnlimitedCell<F>,
    rhs_modulus: AllocatedUnlimitedCell<F>,
    size_modulus: AllocatedUnlimitedCell<F>,

    rhs_round: AllocatedCommonRangeCell<F>,
    rhs_rem: AllocatedCommonRangeCell<F>,
    rhs_rem_diff: AllocatedCommonRangeCell<F>,

    is_i32: AllocatedBitCell<F>,

    is_shl: AllocatedBitCell<F>,
    is_shr_u: AllocatedBitCell<F>,
    is_shr_s: AllocatedBitCell<F>,
    is_rotl: AllocatedBitCell<F>,
    is_rotr: AllocatedBitCell<F>,
    is_l: AllocatedBitCell<F>,
    is_r: AllocatedBitCell<F>,

    degree_helper: AllocatedUnlimitedCell<F>,
    lookup_pow_modulus: AllocatedUnlimitedCell<F>,
    lookup_pow_power: AllocatedUnlimitedCell<F>,

    memory_table_lookup_stack_read_lhs: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_rhs: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct BinShiftConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BinShiftConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_i32 = allocator.alloc_bit_cell();
        let lhs = allocator
            .alloc_u64_with_flag_bit_cell_dyn(constraint_builder, move |meta| is_i32.expr(meta));
        let rhs = allocator.alloc_u64_cell();
        let round = allocator.alloc_u64_cell();
        let rem = allocator.alloc_u64_cell();
        let diff = allocator.alloc_u64_cell();
        let pad = allocator.alloc_unlimited_cell();
        let rhs_modulus = allocator.alloc_unlimited_cell();
        let size_modulus = allocator.alloc_unlimited_cell();

        let rhs_round = allocator.alloc_common_range_cell();
        let rhs_rem = allocator.alloc_common_range_cell();
        let rhs_rem_diff = allocator.alloc_common_range_cell();

        let is_shl = allocator.alloc_bit_cell();
        let is_shr_u = allocator.alloc_bit_cell();
        let is_shr_s = allocator.alloc_bit_cell();
        let is_rotl = allocator.alloc_bit_cell();
        let is_rotr = allocator.alloc_bit_cell();

        let is_l = allocator.alloc_bit_cell();
        let is_r = allocator.alloc_bit_cell();

        let degree_helper = allocator.alloc_unlimited_cell();

        let lookup_pow_modulus = common_config.pow_table_lookup_modulus_cell;
        let lookup_pow_power = common_config.pow_table_lookup_power_cell;

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_rhs = allocator.alloc_memory_table_lookup_read_cell(
            "op_bin_shift stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| rhs.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_lhs = allocator.alloc_memory_table_lookup_read_cell(
            "op_bin_shift stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |meta| is_i32.expr(meta),
            move |meta| lhs.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator
            .alloc_memory_table_lookup_write_cell_with_value(
                "op_bin_shift stack write",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(2),
                move |meta| is_i32.expr(meta),
                move |____| constant_from!(1),
            );
        let res = memory_table_lookup_stack_write.value_cell;

        constraint_builder.push(
            "bin_shift op select",
            Box::new(move |meta| {
                vec![
                    is_shr_u.expr(meta) + is_shr_s.expr(meta) + is_rotr.expr(meta)
                        - is_r.expr(meta),
                    is_shl.expr(meta) + is_rotl.expr(meta) - is_l.expr(meta),
                    is_l.expr(meta) + is_r.expr(meta) - constant_from!(1),
                ]
            }),
        );

        // cs 1: rhs_modulus = if is_i32 { 32 } else { 64 }
        // cs 2: size_modulus = 1 << rhs_modulus
        constraint_builder.push(
            "bin_shift modulus",
            Box::new(move |meta| {
                vec![
                    rhs_modulus.expr(meta) - constant_from!(64)
                        + is_i32.expr(meta) * constant_from!(32),
                    size_modulus.expr(meta) - constant_from_bn!(&(BigUint::from(1u64) << 64usize))
                        + is_i32.expr(meta) * constant_from!((u32::MAX as u64) << 32),
                ]
            }),
        );

        // cs 3: (rhs_round, rhs_rem) = (rhs & 0xffff) div rhs_modulus
        // cs 3.helper: rhs_rem < rhs_modulus
        constraint_builder.push(
            "bin_shift rhs rem",
            Box::new(move |meta| {
                vec![
                    rhs_round.expr(meta) * rhs_modulus.expr(meta) + rhs_rem.expr(meta)
                        - rhs.u16_cells_le[0].expr(meta),
                    rhs_rem.expr(meta) + rhs_rem_diff.expr(meta) + constant_from!(1)
                        - rhs_modulus.expr(meta),
                ]
            }),
        );

        // cs 4: lookup_pow_modulus = 1 << rhs_rem
        constraint_builder.push(
            "bin_shift modulus pow lookup",
            Box::new(move |meta| {
                vec![lookup_pow_power.expr(meta) - pow_table_power_encode(rhs_rem.expr(meta))]
            }),
        );

        // cs is_r:
        // 1: (round, rem) = lhs div lookup_pow_modulus
        // 1.helper: rem < lookup_pow_modulus
        constraint_builder.push(
            "bin_shift is_r",
            Box::new(move |meta| {
                vec![
                    is_r.expr(meta)
                        * (rem.u64_cell.expr(meta)
                            + round.u64_cell.expr(meta) * lookup_pow_modulus.expr(meta)
                            - lhs.u64_cell.expr(meta)),
                    is_r.expr(meta)
                        * (rem.u64_cell.expr(meta) + diff.u64_cell.expr(meta) + constant_from!(1)
                            - lookup_pow_modulus.expr(meta)),
                ]
            }),
        );

        // cs is_shr_u:
        // 2: res = round
        constraint_builder.push(
            "bin_shift shr_u",
            Box::new(move |meta| {
                vec![is_shr_u.expr(meta) * (res.expr(meta) - round.u64_cell.expr(meta))]
            }),
        );

        // cs is_shr_s:
        // let size = if is_i32 { 32 } else { 64 }
        // 1. pad = flag * ((1 << rhs_rem) - 1)) << (size - rhs_rem)
        // 2: res = pad + round
        constraint_builder.push(
            "bin_shift shr_s",
            Box::new(move |meta| {
                vec![
                    degree_helper.expr(meta)
                        - (lookup_pow_modulus.expr(meta) - constant_from!(1))
                            * size_modulus.expr(meta),
                    is_shr_s.expr(meta)
                        * (pad.expr(meta) * lookup_pow_modulus.expr(meta)
                            - lhs.flag_bit_cell.expr(meta) * degree_helper.expr(meta)),
                    is_shr_s.expr(meta)
                        * (res.expr(meta) - round.u64_cell.expr(meta) - pad.expr(meta)),
                ]
            }),
        );

        // cs is_rotr:
        // 1: res = round + rem * size_modulus / lookup_pow_modulus
        constraint_builder.push(
            "bin_shift rotr",
            Box::new(move |meta| {
                vec![
                    is_rotr.expr(meta)
                        * (res.expr(meta) * lookup_pow_modulus.expr(meta)
                            - round.u64_cell.expr(meta) * lookup_pow_modulus.expr(meta)
                            - rem.u64_cell.expr(meta) * size_modulus.expr(meta)),
                ]
            }),
        );

        // cs is_l:
        // 1: (round, rem) = (lhs << rhs_rem) div size_modulus
        // 1.helper: rem < size_modulus
        constraint_builder.push(
            "bin_shift shl",
            Box::new(move |meta| {
                vec![
                    is_l.expr(meta)
                        * (lhs.u64_cell.expr(meta) * lookup_pow_modulus.expr(meta)
                            - round.u64_cell.expr(meta) * size_modulus.expr(meta)
                            - rem.u64_cell.expr(meta)),
                    is_l.expr(meta)
                        * (rem.u64_cell.expr(meta) + diff.u64_cell.expr(meta) + constant_from!(1)
                            - size_modulus.expr(meta)),
                ]
            }),
        );

        // cs is_shl:
        // 1: res = rem
        constraint_builder.push(
            "bin_shift shl",
            Box::new(move |meta| {
                vec![is_shl.expr(meta) * (res.expr(meta) - rem.u64_cell.expr(meta))]
            }),
        );

        // cs is_rotl:
        // 2: res = rem + round
        constraint_builder.push(
            "bin_shift rotl",
            Box::new(move |meta| {
                vec![
                    is_rotl.expr(meta)
                        * (res.expr(meta) - rem.u64_cell.expr(meta) - round.u64_cell.expr(meta)),
                ]
            }),
        );

        Box::new(BinShiftConfig {
            lhs,
            rhs,
            round,
            rem,
            diff,
            pad,
            res,
            rhs_round,
            rhs_rem,
            rhs_rem_diff,
            is_i32,
            is_shl,
            is_shr_u,
            is_shr_s,
            is_rotl,
            is_rotr,
            is_l,
            is_r,
            lookup_pow_modulus,
            lookup_pow_power,
            memory_table_lookup_stack_read_lhs,
            memory_table_lookup_stack_read_rhs,
            memory_table_lookup_stack_write,
            rhs_modulus,
            size_modulus,
            degree_helper,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BinShiftConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BinShift as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_shl.expr(meta)
            * constant!(bn_to_field(
                &(BigUint::from(ShiftOp::Shl as u64) << OPCODE_ARG0_SHIFT)
            ))
            + self.is_shr_u.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::UnsignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_shr_s.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::SignedShr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_rotl.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::Rotl as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_rotr.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(ShiftOp::Rotr as u64) << OPCODE_ARG0_SHIFT)
                ))
            + self.is_i32.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        let (class, left, right, value, power, is_eight_bytes, _is_sign) =
            match entry.eentry.step_info {
                StepInfo::I32BinShiftOp {
                    class,
                    left,
                    right,
                    value,
                } => {
                    let left = left as u32 as u64;
                    let right = right as u32 as u64;
                    let value = value as u32 as u64;
                    let power = right % 32;
                    let is_eight_bytes = false;
                    let is_sign = true;
                    (class, left, right, value, power, is_eight_bytes, is_sign)
                }

                StepInfo::I64BinShiftOp {
                    class,
                    left,
                    right,
                    value,
                } => {
                    let left = left as u64;
                    let right = right as u64;
                    let value = value as u64;
                    let power = right % 64;
                    let is_eight_bytes = true;
                    let is_sign = true;
                    (class, left, right, value, power, is_eight_bytes, is_sign)
                }

                _ => {
                    unreachable!();
                }
            };

        let size = if is_eight_bytes { 64 } else { 32 };
        let size_mask = if is_eight_bytes {
            u64::MAX
        } else {
            u32::MAX as u64
        };

        let modulus = 1u64 << power;
        let size_modulus = if is_eight_bytes {
            BigUint::from(1u64) << 64usize
        } else {
            BigUint::from(1u64) << 32usize
        };

        self.lhs.assign(ctx, left, !is_eight_bytes)?;
        self.rhs.assign(ctx, right)?;
        self.rhs_round
            .assign(ctx, F::from((right & 0xffff) / size))?;
        self.rhs_rem.assign(ctx, F::from(power))?;
        self.rhs_rem_diff.assign(ctx, F::from(size - 1 - power))?;
        self.lookup_pow_modulus.assign(ctx, modulus.into())?;
        self.lookup_pow_power
            .assign_bn(ctx, &pow_table_power_encode(BigUint::from(power)))?;
        self.is_i32
            .assign(ctx, if is_eight_bytes { F::zero() } else { F::one() })?;
        self.res.assign(ctx, F::from(value))?;
        self.rhs_modulus
            .assign_u32(ctx, if is_eight_bytes { 64 } else { 32 })?;
        self.size_modulus.assign_bn(ctx, &size_modulus)?;
        self.degree_helper
            .assign_bn(ctx, &(size_modulus * (modulus - 1)))?;

        match class {
            ShiftOp::Shl => {
                self.is_l.assign(ctx, 1.into())?;
                self.is_shl.assign(ctx, 1.into())?;
                if power != 0 {
                    self.round.assign(ctx, left >> (size - power))?;
                } else {
                    self.round.assign(ctx, 0)?;
                }
                let rem = (left << power) & size_mask;
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, size_mask - rem)?;
            }
            ShiftOp::UnsignedShr => {
                self.is_r.assign(ctx, 1.into())?;
                self.is_shr_u.assign(ctx, 1.into())?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - rem - 1)?;
            }
            ShiftOp::SignedShr => {
                self.is_r.assign(ctx, 1.into())?;
                self.is_shr_s.assign(ctx, 1.into())?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - 1 - rem)?;

                let flag_bit = if is_eight_bytes {
                    left >> 63
                } else {
                    left >> 31
                };
                if flag_bit == 1 && power != 0 {
                    self.pad
                        .assign(ctx, (((1 << power) - 1) << (size - power)).into())?;
                }
            }
            ShiftOp::Rotl => {
                // same as shl
                self.is_l.assign(ctx, 1.into())?;
                self.is_rotl.assign(ctx, 1.into())?;
                if power != 0 {
                    self.round.assign(ctx, left >> (size - power))?;
                } else {
                    self.round.assign(ctx, 0)?;
                }
                let rem = (left << power) & size_mask;
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, size_mask - rem)?;
            }
            ShiftOp::Rotr => {
                // same as shr_u
                self.is_r.assign(ctx, 1.into())?;
                self.is_rotr.assign(ctx, 1.into())?;
                self.round.assign(ctx, left >> power)?;
                let rem = left & ((1 << power) - 1);
                self.rem.assign(ctx, rem)?;
                self.diff.assign(ctx, (1u64 << power) - rem - 1)?;
            }
        }

        self.memory_table_lookup_stack_read_rhs.assign(
            ctx,
            entry.memory_rw_entires[0].start_eid,
            step.current.eid,
            entry.memory_rw_entires[0].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            !is_eight_bytes,
            right,
        )?;

        self.memory_table_lookup_stack_read_lhs.assign(
            ctx,
            entry.memory_rw_entires[1].start_eid,
            step.current.eid,
            entry.memory_rw_entires[1].end_eid,
            step.current.sp + 2,
            LocationType::Stack,
            !is_eight_bytes,
            left,
        )?;

        self.memory_table_lookup_stack_write.assign(
            ctx,
            step.current.eid,
            entry.memory_rw_entires[2].end_eid,
            step.current.sp + 2,
            LocationType::Stack,
            !is_eight_bytes,
            value,
        )?;

        Ok(())
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }
}
