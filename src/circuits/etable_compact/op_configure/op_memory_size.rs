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

pub struct MemorySizeConfig {
    memory_size: CommonRangeCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct MemorySizeConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for MemorySizeConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let memory_size = common.allocated_memory_pages_cell();
        let lookup_stack_write = common.alloc_mtable_lookup();

        Box::new(MemorySizeConfig {
            memory_size,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for MemorySizeConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::MemorySize as u64) << OPCODE_CLASS_SHIFT)
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::MemorySize => {
                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp as u64),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(step_info.current.allocated_memory_pages),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(-F::one()))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta),
                constant_from!(VarType::I32 as u64),
                self.memory_size.expr(meta),
            )),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_memory_size() {
        let textual_repr = r#"
                (module
                    (memory 2)

                    (func (export "test")
                      (memory.size)
                      (drop)
                    )
                   )
                "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
