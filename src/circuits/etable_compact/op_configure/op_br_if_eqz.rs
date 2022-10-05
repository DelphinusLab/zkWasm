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
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{encode::opcode::encode_br_if_eqz, itable::OPCODE_ARG1_SHIFT};
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BrIfEqzConfig {
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

pub struct BrIfEqzConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrIfEqzConfigBuilder {
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
            "op_br_if_eqz cond bit",
            Box::new(move |meta| {
                vec![
                    cond_is_zero.expr(meta) * cond.expr(meta),
                    cond_is_zero.expr(meta) + cond.expr(meta) * cond_inv.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        Box::new(BrIfEqzConfig {
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

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrIfEqzConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_br_if_eqz(
            self.drop.expr(meta),
            self.keep.expr(meta),
            self.dst_pc.expr(meta),
        )
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::BrIfEqz {
                condition,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                let drop: u16 = (*drop).try_into().unwrap();
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

                self.drop.assign(ctx, drop)?;

                if keep.len() > 0 {
                    let keep_type: VarType = keep[0].into();

                    self.keep.assign(ctx, true)?;
                    self.keep_value.assign(ctx, keep_values[0])?;
                    self.keep_type.assign(ctx, keep_type as u16)?;

                    if *condition == 0 {
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
                                BigUint::from(step_info.current.sp + 2 + drop as u64),
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

                self.dst_pc.assign(ctx, (*dst_pc).try_into().unwrap())?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BrIf
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(
            constant_from!(1)
                + constant_from!(2) * self.cond_is_zero.expr(meta) * self.keep.expr(meta),
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
                self.cond_is_zero.expr(meta)
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
                self.cond_is_zero.expr(meta)
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
        Some(constant_from!(1) + self.cond_is_zero.expr(meta) * self.drop.expr(meta))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(
            self.cond_is_zero.expr(meta) * self.dst_pc.expr(meta)
                + (constant_from!(1) - self.cond_is_zero.expr(meta))
                    * (common_config.iid(meta) + constant_from!(1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_br_if_eqz_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (if (i32.const 0)
                  (then (br 0))
                  (else (br 0))
                )
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
