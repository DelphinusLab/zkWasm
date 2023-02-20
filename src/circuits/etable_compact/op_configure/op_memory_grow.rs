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
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct MemoryGrowConfig {
    grow_size: U64Cell,
    result: U64Cell,
    success: BitCell,
    current_maximal_diff: CommonRangeCell,
    lookup_stack_read: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct MemoryGrowConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for MemoryGrowConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let current_memory_size = common.allocated_memory_pages_cell();
        let current_maximal_diff = common.alloc_common_range_value();

        let grow_size = common.alloc_u64();
        let result = common.alloc_u64();
        let success = common.alloc_bit_value();

        let lookup_stack_read = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        let maximal_memory_pages = common.config.circuit_configure.maximal_memory_pages;

        constraint_builder.push(
            "memory_grow: return value",
            Box::new(move |meta| {
                vec![
                    (constant_from!(1) - success.expr(meta))
                        * (result.expr(meta) - constant_from!(u32::MAX)),
                    success.expr(meta) * (result.expr(meta) - current_memory_size.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "memory_grow: updated memory size should less or equal than maximal memory size",
            Box::new(move |meta| {
                vec![
                    (current_memory_size.expr(meta)
                        + grow_size.expr(meta)
                        + current_maximal_diff.expr(meta)
                        - constant_from!(maximal_memory_pages))
                        * success.expr(meta),
                ]
            }),
        );

        Box::new(MemoryGrowConfig {
            grow_size,
            success,
            result,
            current_maximal_diff,
            lookup_stack_read,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for MemoryGrowConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::MemoryGrow as u64) << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::MemoryGrow { grow_size, result } => {
                self.grow_size.assign(ctx, *grow_size as u64)?;
                self.result.assign(ctx, *result as u32 as u64)?;
                self.success.assign(ctx, *result != -1)?;

                if *result != -1 {
                    self.current_maximal_diff.assign(
                        ctx,
                        CommonRange::from(
                            step_info.configure.maximal_memory_pages
                                - (*step_info.current.allocated_memory_pages + (*grow_size as u32)),
                        ),
                    )?;
                }

                self.lookup_stack_read.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(*grow_size as u32),
                    ),
                )?;

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(*result as u32),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
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
                constant_from!(VarType::I32 as u64),
                self.grow_size.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32 as u64),
                self.result.expr(meta),
            )),
            _ => None,
        }
    }

    fn allocated_memory_pages_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.success.expr(meta) * self.grow_size.expr(meta))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_memory_grow() {
        let textual_repr = r#"
                (module
                    (memory 1 2)

                    (func (export "test")
                      (memory.grow (i32.const 1))
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_memory_grow_fail() {
        let textual_repr = r#"
                (module
                    (memory 1 2)

                    (func (export "test")
                      (memory.grow (i32.const 2))
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }

    #[test]
    fn test_memory_grow_lazy_init() {
        let textual_repr = r#"
                (module
                    (memory 0 1)

                    (func (export "test")
                      (memory.grow (i32.const 1))
                      (drop)
                      (i32.const 0)
                      (i32.load offset=0)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
