use super::*;
use crate::circuits::jtable::expression::JtableLookupEntryEncode;
use crate::circuits::mtable_compact::expression::MtableLookupEntryEncode;
use crate::{
    circuits::utils::{bn_to_field, Context},
    constant,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct ReturnConfig {
    keep: BitCell,
    drop: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
}

pub struct ReturnConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ReturnConfigBuilder {
    fn configure(
        _meta: &mut ConstraintSystem<F>,
        common: &mut EventTableCellAllocator<F>,
        _enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let drop = common.alloc_common_range_value();
        let keep = common.alloc_bit_value();
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();

        Box::new(ReturnConfig {
            keep,
            drop,
            vtype,
            value,
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

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
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

    fn jops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2) * self.keep.expr(meta))
    }

    fn next_last_jump_eid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        todo!()
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
            todo!(),
            todo!(),
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
                    * MemoryTableConfig::encode_stack_read(
                        common.eid(meta),
                        constant_from!(1),
                        common.sp(meta) + constant_from!(1),
                        self.vtype.expr(meta),
                        self.value.expr(meta),
                    ),
            ),
            MLookupItem::Second => Some(
                self.keep.expr(meta)
                    * MemoryTableConfig::encode_stack_write(
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
