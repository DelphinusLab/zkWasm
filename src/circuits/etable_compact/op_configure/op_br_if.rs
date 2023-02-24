use super::*;
use crate::{
    circuits::{
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BrIfConfig {
    cond: U64Cell,
    cond_inv: UnlimitedCell,
    cond_is_zero: BitCell,
    keep: BitCell,
    keep_value: U64Cell,
    keep_type: CommonRangeCell,
    drop: CommonRangeCell,
    dst_pc: CommonRangeCell,
    lookup_stack_read_cond: MTableLookupCell,
    lookup_stack_read_return_value: MTableLookupCell,
    lookup_stack_write_return_value: MTableLookupCell,
}

pub struct BrIfConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrIfConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond = common.alloc_u64();
        let cond_inv = common.alloc_unlimited_value();
        let cond_is_zero = common.alloc_bit_value();
        let keep = common.alloc_bit_value();
        let keep_value = common.alloc_u64();
        let keep_type = common.alloc_common_range_value();
        let drop = common.alloc_common_range_value();
        let dst_pc = common.alloc_common_range_value();
        let lookup_stack_read_cond = common.alloc_mtable_lookup();
        let lookup_stack_read_return_value = common.alloc_mtable_lookup();
        let lookup_stack_write_return_value = common.alloc_mtable_lookup();

        constraint_builder.push(
            "op_br_if cond bit",
            Box::new(move |meta| {
                vec![
                    cond_is_zero.expr(meta) * cond.expr(meta),
                    cond_is_zero.expr(meta) + cond.expr(meta) * cond_inv.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        Box::new(BrIfConfig {
            cond,
            cond_inv,
            keep,
            keep_value,
            keep_type,
            drop,
            dst_pc,
            lookup_stack_read_cond,
            lookup_stack_read_return_value,
            lookup_stack_write_return_value,
            cond_is_zero,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrIfConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::BrIf as u64) << OPCODE_CLASS_SHIFT)
        )) + self.drop.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.keep.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.dst_pc.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::BrIfNez {
                condition,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                let cond = *condition as u32 as u64;

                self.lookup_stack_read_cond.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(entry.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(entry.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(cond),
                    ),
                )?;

                self.drop.assign(ctx, F::from(*drop as u64))?;

                if keep.len() > 0 {
                    let keep_type: VarType = keep[0].into();

                    self.keep.assign(ctx, true)?;
                    self.keep_value.assign(ctx, keep_values[0])?;
                    self.keep_type.assign(ctx, F::from(keep_type as u64))?;

                    if *condition != 0 {
                        self.lookup_stack_read_return_value.assign(
                            ctx,
                            &MemoryTableLookupEncode::encode_stack_read(
                                BigUint::from(entry.eid),
                                BigUint::from(2 as u64),
                                BigUint::from(entry.sp + 2),
                                BigUint::from(keep_type as u16),
                                BigUint::from(keep_values[0]),
                            ),
                        )?;

                        self.lookup_stack_write_return_value.assign(
                            ctx,
                            &MemoryTableLookupEncode::encode_stack_write(
                                BigUint::from(step_info.current.eid),
                                BigUint::from(3 as u64),
                                BigUint::from(step_info.current.sp + 2 + drop),
                                BigUint::from(keep_type as u16),
                                BigUint::from(keep_values[0]),
                            ),
                        )?;
                    }
                }

                self.cond.assign(ctx, cond)?;
                self.cond_inv
                    .assign(ctx, F::from(cond).invert().unwrap_or(F::zero()))?;
                self.cond_is_zero.assign(ctx, cond == 0)?;

                self.dst_pc.assign(ctx, F::from((*dst_pc) as u64))?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> u64 {
        match &entry.step_info {
            StepInfo::BrIfNez {
                condition, keep, ..
            } => {
                if *condition != 0 && keep.len() > 0 {
                    assert!(keep.len() == 1);
                    2
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            constant_from!(1)
                + constant_from!(2)
                    * (constant_from!(1) - self.cond_is_zero.expr(meta))
                    * self.keep.expr(meta),
        )
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32 as u32 as u64),
                self.cond.expr(meta),
            )),

            MLookupItem::Second => Some(
                (constant_from!(1) - self.cond_is_zero.expr(meta))
                    * self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(2),
                        common_config.sp(meta) + constant_from!(2),
                        self.keep_type.expr(meta),
                        self.keep_value.expr(meta),
                    ),
            ),

            MLookupItem::Third => Some(
                (constant_from!(1) - self.cond_is_zero.expr(meta))
                    * self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_write(
                        common_config.eid(meta),
                        constant_from!(3),
                        common_config.sp(meta) + constant_from!(2) + self.drop.expr(meta),
                        self.keep_type.expr(meta),
                        self.keep_value.expr(meta),
                    ),
            ),

            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            constant_from!(1)
                + (constant_from!(1) - self.cond_is_zero.expr(meta)) * self.drop.expr(meta),
        )
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(
            (constant_from!(1) - self.cond_is_zero.expr(meta)) * self.dst_pc.expr(meta)
                + self.cond_is_zero.expr(meta) * (common_config.iid(meta) + constant_from!(1)),
        )
    }
}
