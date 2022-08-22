use super::*;
use crate::circuits::jtable::expression::{
    JtableLookupEntryEncode, EID_SHIFT, FID_SHIFT, LAST_JUMP_EID_SHIFT, MOID_SHIFT,
};
use crate::circuits::mtable_compact::encode::MemoryTableLookupEncode;
use crate::{
    circuits::utils::{bn_to_field, Context},
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::ToBigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct ReturnConfig {
    keep: BitCell,
    drop: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    return_lookup: JTableLookupCell,
}

pub struct ReturnConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ReturnConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let drop = common.alloc_common_range_value();
        let keep = common.alloc_bit_value();
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let return_lookup = common.alloc_jtable_lookup();

        Box::new(ReturnConfig {
            keep,
            drop,
            vtype,
            value,
            return_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ReturnConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Return as u64) << OPCODE_CLASS_SHIFT)
        )) + self.drop.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.keep.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.vtype.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            specs::step::StepInfo::Return {
                drop,
                keep,
                keep_values,
                ..
            } => {
                assert!(keep.len() <= 1);
                assert!(*drop < 1 << 16);
                assert_eq!(keep.len(), keep_values.len());

                self.drop.assign(ctx, *drop as u16)?;

                if keep_values.len() == 0 {
                    self.keep.assign(ctx, false)?;
                } else {
                    self.keep.assign(ctx, true)?;
                    self.vtype.assign(ctx, keep[0] as u16)?;
                    self.value.assign(ctx, keep_values[0])?;
                }

                {
                    let one = BigUint::from(1u64);

                    let value: BigUint = step.current.last_jump_eid.to_biguint().unwrap()
                        * (&one << EID_SHIFT)
                        + step.next.last_jump_eid.to_biguint().unwrap()
                            * (&one << LAST_JUMP_EID_SHIFT)
                        + step.next.moid.to_biguint().unwrap() * (&one << MOID_SHIFT)
                        + step.next.fid.to_biguint().unwrap() * (&one << FID_SHIFT)
                        + step.next.iid.to_biguint().unwrap();

                    self.return_lookup.assign(ctx, &value)?;
                }

                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Return
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.drop.expr(meta))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2) * self.keep.expr(meta))
    }

    fn next_last_jump_eid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.next_last_jump_eid(meta))
    }

    fn next_moid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.next_moid(meta))
    }

    fn next_fid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.next_fid(meta))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.next_iid(meta))
    }

    fn jtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(JumpTableConfig::encode_lookup(
            common_config.last_jump_eid(meta),
            common_config.next_last_jump_eid(meta),
            common_config.next_moid(meta),
            common_config.next_fid(meta),
            common_config.next_iid(meta),
        ))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(
                self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_read(
                        common.eid(meta),
                        constant_from!(1),
                        common.sp(meta) + constant_from!(1),
                        self.vtype.expr(meta),
                        self.value.expr(meta),
                    ),
            ),
            MLookupItem::Second => Some(
                self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_write(
                        common.eid(meta),
                        constant_from!(2),
                        common.sp(meta) + self.drop.expr(meta) + constant_from!(1),
                        self.vtype.expr(meta),
                        self.value.expr(meta),
                    ),
            ),
            MLookupItem::Third => None,
            MLookupItem::Fourth => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_trivial_return() {
        let textual_repr = r#"
            (module
                (func (export "test")
                  return
                )
               )
            "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_return_with_drop_ok() {
        let textual_repr = r#"
            (module
                (func (export "test")
                  (block
                    (i32.const 0)
                    (i32.const 0)
                    return
                  )
                )
               )
            "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
