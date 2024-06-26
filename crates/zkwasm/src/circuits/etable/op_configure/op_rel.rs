use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use specs::itable::RelOp;
use specs::itable::OPCODE_ARG0_SHIFT;
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct RelConfig<F: FieldExt> {
    is_i32: AllocatedBitCell<F>,

    lhs: AllocatedU64CellWithFlagBitDynSign<F>,
    rhs: AllocatedU64CellWithFlagBitDynSign<F>,

    diff: AllocatedU64Cell<F>,
    diff_inv: AllocatedUnlimitedCell<F>,

    res: AllocatedUnlimitedCell<F>,

    res_is_eq: AllocatedBitCell<F>,
    res_is_lt: AllocatedBitCell<F>,
    res_is_gt: AllocatedBitCell<F>,

    op_is_eq: AllocatedBitCell<F>,
    op_is_ne: AllocatedBitCell<F>,
    op_is_lt: AllocatedBitCell<F>,
    op_is_gt: AllocatedBitCell<F>,
    op_is_le: AllocatedBitCell<F>,
    op_is_ge: AllocatedBitCell<F>,
    op_is_sign: AllocatedBitCell<F>,

    l_pos_r_pos: AllocatedUnlimitedCell<F>,
    l_pos_r_neg: AllocatedUnlimitedCell<F>,
    l_neg_r_pos: AllocatedUnlimitedCell<F>,
    l_neg_r_neg: AllocatedUnlimitedCell<F>,

    memory_table_lookup_stack_read_lhs: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_rhs: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct RelConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for RelConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let is_i32 = allocator.alloc_bit_cell();
        let op_is_sign = allocator.alloc_bit_cell();

        let lhs = allocator.alloc_u64_with_flag_bit_cell_dyn_sign(
            constraint_builder,
            move |meta| is_i32.expr(meta),
            move |meta| op_is_sign.expr(meta),
        );
        let rhs = allocator.alloc_u64_with_flag_bit_cell_dyn_sign(
            constraint_builder,
            move |meta| is_i32.expr(meta),
            move |meta| op_is_sign.expr(meta),
        );

        let diff = allocator.alloc_u64_cell();
        let diff_inv = allocator.alloc_unlimited_cell();
        let res = allocator.alloc_unlimited_cell();

        let res_is_eq = allocator.alloc_bit_cell();
        let res_is_lt = allocator.alloc_bit_cell();
        let res_is_gt = allocator.alloc_bit_cell();

        let op_is_eq = allocator.alloc_bit_cell();
        let op_is_ne = allocator.alloc_bit_cell();
        let op_is_lt = allocator.alloc_bit_cell();
        let op_is_gt = allocator.alloc_bit_cell();
        let op_is_le = allocator.alloc_bit_cell();
        let op_is_ge = allocator.alloc_bit_cell();

        constraint_builder.push(
            "rel: selector",
            Box::new(move |meta| {
                vec![
                    (op_is_eq.expr(meta)
                        + op_is_ne.expr(meta)
                        + op_is_lt.expr(meta)
                        + op_is_gt.expr(meta)
                        + op_is_le.expr(meta)
                        + op_is_ge.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        constraint_builder.push(
            "rel: compare diff",
            Box::new(move |meta| {
                vec![
                    (lhs.u64_cell.expr(meta) + res_is_lt.expr(meta) * diff.u64_cell.expr(meta)
                        - res_is_gt.expr(meta) * diff.u64_cell.expr(meta)
                        - rhs.u64_cell.expr(meta)),
                    (res_is_gt.expr(meta) + res_is_lt.expr(meta) + res_is_eq.expr(meta)
                        - constant_from!(1)),
                    (diff.u64_cell.expr(meta) * res_is_eq.expr(meta)),
                    (diff.u64_cell.expr(meta) * diff_inv.expr(meta) + res_is_eq.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        let l_pos_r_pos = allocator.alloc_unlimited_cell();
        let l_pos_r_neg = allocator.alloc_unlimited_cell();
        let l_neg_r_pos = allocator.alloc_unlimited_cell();
        let l_neg_r_neg = allocator.alloc_unlimited_cell();

        constraint_builder.push(
            "rel: compare op res",
            Box::new(move |meta| {
                vec![
                    l_pos_r_pos.expr(meta)
                        - ((constant_from!(1) - lhs.flag_bit_cell.expr(meta))
                            * (constant_from!(1) - rhs.flag_bit_cell.expr(meta))),
                    l_pos_r_neg.expr(meta)
                        - ((constant_from!(1) - lhs.flag_bit_cell.expr(meta))
                            * rhs.flag_bit_cell.expr(meta)),
                    l_neg_r_pos.expr(meta)
                        - (lhs.flag_bit_cell.expr(meta)
                            * (constant_from!(1) - rhs.flag_bit_cell.expr(meta))),
                    l_neg_r_neg.expr(meta)
                        - (lhs.flag_bit_cell.expr(meta) * rhs.flag_bit_cell.expr(meta)),
                    op_is_eq.expr(meta) * (res.expr(meta) - res_is_eq.expr(meta)),
                    op_is_ne.expr(meta)
                        * (res.expr(meta) - constant_from!(1) + res_is_eq.expr(meta)),
                    op_is_lt.expr(meta)
                        * (res.expr(meta)
                            - l_neg_r_pos.expr(meta)
                            - l_pos_r_pos.expr(meta) * res_is_lt.expr(meta)
                            - l_neg_r_neg.expr(meta) * res_is_lt.expr(meta)),
                    op_is_le.expr(meta)
                        * (res.expr(meta)
                            - l_neg_r_pos.expr(meta)
                            - l_pos_r_pos.expr(meta) * res_is_lt.expr(meta)
                            - l_neg_r_neg.expr(meta) * res_is_lt.expr(meta)
                            - res_is_eq.expr(meta)),
                    op_is_gt.expr(meta)
                        * (res.expr(meta)
                            - l_pos_r_neg.expr(meta)
                            - l_pos_r_pos.expr(meta) * res_is_gt.expr(meta)
                            - l_neg_r_neg.expr(meta) * res_is_gt.expr(meta)),
                    op_is_ge.expr(meta)
                        * (res.expr(meta)
                            - l_pos_r_neg.expr(meta)
                            - l_pos_r_pos.expr(meta) * res_is_gt.expr(meta)
                            - l_neg_r_neg.expr(meta) * res_is_gt.expr(meta)
                            - res_is_eq.expr(meta)),
                ]
            }),
        );

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read_rhs = allocator.alloc_memory_table_lookup_read_cell(
            "op_bin stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| rhs.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_lhs = allocator.alloc_memory_table_lookup_read_cell(
            "op_bin stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |meta| is_i32.expr(meta),
            move |meta| lhs.u64_cell.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "op_bin stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |____| constant_from!(1),
            move |meta| res.expr(meta),
            move |____| constant_from!(1),
        );

        Box::new(RelConfig {
            is_i32,
            lhs,
            rhs,
            diff,
            diff_inv,
            res,
            res_is_eq,
            res_is_lt,
            res_is_gt,
            op_is_eq,
            op_is_ne,
            op_is_lt,
            op_is_gt,
            op_is_le,
            op_is_ge,
            op_is_sign,
            l_pos_r_pos,
            l_neg_r_pos,
            l_pos_r_neg,
            l_neg_r_neg,
            memory_table_lookup_stack_read_lhs,
            memory_table_lookup_stack_read_rhs,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for RelConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let subop_eq = |meta: &mut VirtualCells<F>| {
            self.op_is_eq.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::Eq as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_ne = |meta: &mut VirtualCells<F>| {
            self.op_is_ne.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::Ne as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_gt_u = |meta: &mut VirtualCells<F>| {
            self.op_is_gt.expr(meta)
                * (constant_from!(1) - self.op_is_sign.expr(meta))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedGt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_ge_u = |meta: &mut VirtualCells<F>| {
            self.op_is_ge.expr(meta)
                * (constant_from!(1) - self.op_is_sign.expr(meta))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedGe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_lt_u = |meta: &mut VirtualCells<F>| {
            self.op_is_lt.expr(meta)
                * (constant_from!(1) - self.op_is_sign.expr(meta))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedLt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_le_u = |meta: &mut VirtualCells<F>| {
            self.op_is_le.expr(meta)
                * (constant_from!(1) - self.op_is_sign.expr(meta))
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::UnsignedLe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_gt_s = |meta: &mut VirtualCells<F>| {
            self.op_is_gt.expr(meta)
                * self.op_is_sign.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::SignedGt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_ge_s = |meta: &mut VirtualCells<F>| {
            self.op_is_ge.expr(meta)
                * self.op_is_sign.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::SignedGe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_lt_s = |meta: &mut VirtualCells<F>| {
            self.op_is_lt.expr(meta)
                * self.op_is_sign.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::SignedLt as u64) << OPCODE_ARG0_SHIFT)
                ))
        };
        let subop_le_s = |meta: &mut VirtualCells<F>| {
            self.op_is_le.expr(meta)
                * self.op_is_sign.expr(meta)
                * constant!(bn_to_field(
                    &(BigUint::from(RelOp::SignedLe as u64) << OPCODE_ARG0_SHIFT)
                ))
        };

        let subop = |meta: &mut VirtualCells<F>| {
            subop_eq(meta)
                + subop_ne(meta)
                + subop_ge_u(meta)
                + subop_gt_u(meta)
                + subop_le_u(meta)
                + subop_lt_u(meta)
                + subop_ge_s(meta)
                + subop_gt_s(meta)
                + subop_le_s(meta)
                + subop_lt_s(meta)
        };

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Rel as u64) << OPCODE_CLASS_SHIFT)
        )) + subop(meta)
            + self.is_i32.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        let (class, var_type, lhs, rhs, value, diff) = match entry.eentry.step_info {
            StepInfo::I32Comp {
                class,
                left,
                right,
                value,
            } => {
                let var_type = VarType::I32;
                let lhs = left as u32 as u64;
                let rhs = right as u32 as u64;
                let diff = if lhs < rhs { rhs - lhs } else { lhs - rhs };

                (class, var_type, lhs, rhs, value, diff)
            }

            StepInfo::I64Comp {
                class,
                left,
                right,
                value,
            } => {
                let var_type = VarType::I64;
                let lhs = left as u64;
                let rhs = right as u64;
                let diff = if lhs < rhs { rhs - lhs } else { lhs - rhs };

                (class, var_type, lhs, rhs, value, diff)
            }

            _ => unreachable!(),
        };

        if var_type == VarType::I32 {
            self.is_i32.assign(ctx, F::one())?;
        }

        let op_is_sign = vec![
            RelOp::SignedGt,
            RelOp::SignedGe,
            RelOp::SignedLt,
            RelOp::SignedLe,
        ]
        .contains(&class);

        if op_is_sign {
            self.op_is_sign.assign(ctx, F::one())?;
        }

        {
            let (l_neg, r_neg) = if op_is_sign {
                let l_neg = if var_type == VarType::I32 {
                    (lhs as i32).is_negative()
                } else {
                    (lhs as i64).is_negative()
                };
                let r_neg = if var_type == VarType::I32 {
                    (rhs as i32).is_negative()
                } else {
                    (rhs as i64).is_negative()
                };
                (l_neg, r_neg)
            } else {
                (false, false)
            };

            self.l_pos_r_pos.assign(ctx, F::from(!l_neg && !r_neg))?;
            self.l_pos_r_neg.assign(ctx, F::from(!l_neg && r_neg))?;
            self.l_neg_r_pos.assign(ctx, F::from(l_neg && !r_neg))?;
            self.l_neg_r_neg.assign(ctx, F::from(l_neg && r_neg))?;
        }

        self.lhs
            .assign(ctx, lhs, var_type == VarType::I32, op_is_sign)?;
        self.rhs
            .assign(ctx, rhs, var_type == VarType::I32, op_is_sign)?;
        self.diff.assign(ctx, diff)?;

        if diff != 0 {
            self.diff_inv.assign(ctx, step.field_helper.invert(diff))?;
        }
        {
            self.res_is_eq.assign_bool(ctx, lhs == rhs)?;
            self.res_is_gt.assign_bool(ctx, lhs > rhs)?;
            self.res_is_lt.assign_bool(ctx, lhs < rhs)?;
        }
        self.res
            .assign(ctx, if value { F::one() } else { F::zero() })?;

        match class {
            RelOp::Eq => {
                self.op_is_eq.assign(ctx, F::one())?;
            }
            RelOp::Ne => {
                self.op_is_ne.assign(ctx, F::one())?;
            }
            RelOp::SignedGt => {
                self.op_is_gt.assign(ctx, F::one())?;
            }
            RelOp::UnsignedGt => {
                self.op_is_gt.assign(ctx, F::one())?;
            }
            RelOp::SignedGe => {
                self.op_is_ge.assign(ctx, F::one())?;
            }
            RelOp::UnsignedGe => {
                self.op_is_ge.assign(ctx, F::one())?;
            }
            RelOp::SignedLt => {
                self.op_is_lt.assign(ctx, F::one())?;
            }
            RelOp::UnsignedLt => {
                self.op_is_lt.assign(ctx, F::one())?;
            }
            RelOp::SignedLe => {
                self.op_is_le.assign(ctx, F::one())?;
            }
            RelOp::UnsignedLe => {
                self.op_is_le.assign(ctx, F::one())?;
            }
        };

        self.memory_table_lookup_stack_read_rhs.assign(
            ctx,
            entry.memory_rw_entires[0].start_eid,
            step.current.eid,
            entry.memory_rw_entires[0].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            var_type == VarType::I32,
            rhs,
        )?;

        self.memory_table_lookup_stack_read_lhs.assign(
            ctx,
            entry.memory_rw_entires[1].start_eid,
            step.current.eid,
            entry.memory_rw_entires[1].end_eid,
            step.current.sp + 2,
            LocationType::Stack,
            var_type == VarType::I32,
            lhs,
        )?;

        self.memory_table_lookup_stack_write.assign(
            ctx,
            step.current.eid,
            entry.memory_rw_entires[2].end_eid,
            step.current.sp + 2,
            LocationType::Stack,
            true,
            value as u64,
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
