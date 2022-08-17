use super::*;
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
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct LoadConfig {
    load_offset: CommonRangeCell,
    // TODO: U32Cell?
    load_base: U64Cell,
    vtype: CommonRangeCell,
    value: U64Cell,
    mmid: CommonRangeCell,
    // TODO: U32Cell?
    bytes8_address: U64Cell,
    bytes8_offset: CommonRangeCell,
    bytes8_value: U64Cell,
    lookup_stack_read: MTableLookupCell,
    lookup_heap_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct LoadConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LoadConfigBuilder {
    fn configure(
        _meta: &mut ConstraintSystem<F>,
        common: &mut EventTableCellAllocator<F>,
        _enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let load_offset = common.alloc_common_range_value();
        let load_base = common.alloc_u64();
        let vtype = common.alloc_common_range_value();
        let value = common.alloc_u64();
        let mmid = common.alloc_common_range_value();

        let bytes8_address = common.alloc_u64();
        let bytes8_offset = common.alloc_common_range_value();
        let bytes8_value = common.alloc_u64();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_heap_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        // TODO: add more constraints

        Box::new(LoadConfig {
            load_offset,
            load_base,
            vtype,
            value,
            mmid,
            bytes8_address,
            bytes8_offset,
            bytes8_value,
            lookup_stack_read,
            lookup_heap_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LoadConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
        )) + self.vtype.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.load_offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match entry.step_info {
            StepInfo::Load {
                vtype,
                offset,
                raw_address,
                effective_address,
                value,
                block_value,
                mmid,
            } => {
                self.load_base.assign(ctx, raw_address.into())?;
                self.load_offset.assign(ctx, offset.try_into().unwrap())?;
                self.vtype.assign(ctx, vtype as u16)?;
                self.value.assign(ctx, value)?;
                self.mmid.assign(ctx, mmid.try_into().unwrap())?;

                self.bytes8_offset
                    .assign(ctx, (effective_address % 8).try_into().unwrap())?;
                self.bytes8_address
                    .assign(ctx, (effective_address / 8).into())?;
                self.bytes8_value.assign(ctx, block_value)?;

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableConfig::<F>::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(raw_address),
                    ),
                )?;

                self.lookup_heap_read.assign(
                    ctx,
                    &MemoryTableConfig::<F>::encode_memory_load(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(mmid),
                        BigUint::from(step_info.current.sp),
                        BigUint::from(VarType::U64 as u16),
                        BigUint::from(block_value),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableConfig::<F>::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(3 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(vtype as u16),
                        BigUint::from(value),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Load
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(3))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableConfig::<F>::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32),
                self.load_base.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableConfig::<F>::encode_memory_load(
                common_config.eid(meta),
                constant_from!(2),
                self.mmid.expr(meta),
                self.bytes8_address.expr(meta),
                constant_from!(VarType::U64),
                self.bytes8_value.expr(meta),
            )),
            MLookupItem::Third => Some(MemoryTableConfig::<F>::encode_stack_read(
                common_config.eid(meta),
                constant_from!(3),
                common_config.sp(meta) + constant_from!(1),
                self.vtype.expr(meta),
                self.value.expr(meta),
            )),
            MLookupItem::Fourth => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_builder::test_circuit_noexternal;

    #[test]
    fn test_load() {
        let textual_repr = r#"
                (module
                    (memory $0 1)
                    (data (i32.const 0) "\01\00\00\00\01\00\00\00")
                    (func (export "test")
                      (i32.const 0)
                      (i32.load offset=0)
                      (drop)
                      (i32.const 4)
                      (i32.load offset=0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
