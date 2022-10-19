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
use specs::{
    encode::opcode::{encode_global_get, encode_global_set},
    step::StepInfo,
};
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct GlobalSetConfig {
    origin_module: CommonRangeCell,
    origin_idx: CommonRangeCell,
    module: CommonRangeCell,
    idx: CommonRangeCell,
    vtype: CommonRangeCell,
    value: U64Cell,
    lookup_stack_read: MTableLookupCell,
    lookup_global_set: MTableLookupCell,
}

pub struct GlobalSetConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for GlobalSetConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let origin_module = common.alloc_common_range_value();
        let origin_idx = common.alloc_common_range_value();
        let module = common.alloc_common_range_value();
        let idx = common.alloc_common_range_value();

        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_global_set = common.alloc_mtable_lookup();

        // TODO: constraints
        // build relation between (origin_module, origin_idx) and (module, idx)

        Box::new(GlobalSetConfig {
            origin_module,
            origin_idx,
            module,
            idx,
            vtype,
            value,
            lookup_stack_read,
            lookup_global_set,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for GlobalSetConfig {
    // TODO: Is it necessary to encode vtype
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_global_set(self.idx.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::SetGlobal {
                idx,
                origin_module,
                origin_idx,
                vtype,
                value,
                ..
            } => {
                self.idx.assign(ctx, *idx as u16)?;
                self.module.assign(ctx, step_info.current.moid)?;
                self.origin_idx.assign(ctx, *origin_idx as u16)?;
                self.origin_module.assign(ctx, *origin_module as u16)?;
                self.vtype.assign(ctx, *vtype as u16)?;
                self.value.assign(ctx, *value)?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(*vtype as u16),
                        BigUint::from(*value),
                    ),
                )?;

                self.lookup_global_set.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_global_set(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(*origin_module as u64),
                        BigUint::from(*origin_idx as u64),
                        BigUint::from(*vtype as u64),
                        BigUint::from(*value),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        // Delete opcode_class
        OpcodeClass::GlobalSet
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
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
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_global_set(
                common_config.eid(meta),
                constant_from!(2),
                self.origin_module.expr(meta),
                self.origin_idx.expr(meta),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_global_set() {
        let textual_repr = r#"
                (module
                    (global $global_i32 (mut i32) (i32.const 10))

                    (func (export "test")
                        (i32.const 0)
                        (global.set $global_i32)
                    )
                )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
