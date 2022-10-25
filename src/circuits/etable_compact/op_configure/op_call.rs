use super::*;
use crate::circuits::utils::Context;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::step::StepInfo;
use specs::{encode::opcode::encode_call, etable::EventTableEntry, itable::OpcodeClass};

pub struct CallConfig {
    index: CommonRangeCell,
}

pub struct CallConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let index = common.alloc_common_range_value();

        Box::new(CallConfig { index })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call(self.index.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        _step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::Call { index } => {
                self.index.assign(ctx, *index)?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Call
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(0))
    }

    fn jops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn next_last_jump_eid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.next_last_jump_eid(meta))
    }

    fn next_fid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.index.expr(meta))
    }

    fn next_iid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(constant_from!(0))
    }
}

#[cfg(test)]

mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_call() {
        let textual_repr = r#"
        (module
            (func $foo (param i32) (result i32)
            (local i64 i32)
              i32.const 0
            )
            (func (export "test")
              (i32.const 0)
              call $foo
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
