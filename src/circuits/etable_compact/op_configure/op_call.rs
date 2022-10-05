use super::*;
use crate::{
    circuits::{
        intable::InputTableEncode,
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
    encode::opcode::encode_call,
    etable::EventTableEntry,
    host_function::HostPlugin,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct CallConfig {
    index: CommonRangeCell,
}

pub struct CallConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        // TODO
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
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::Call { index } => {
                // TODO

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

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            _ => None,
        }
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
