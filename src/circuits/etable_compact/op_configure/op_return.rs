use super::*;
use crate::circuits::utils::Context;
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Error, Expression, VirtualCells},
};
use specs::{etable::EventTableEntry, itable::OpcodeClass};

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
        todo!()
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        todo!()
    }

    fn opcode_class(&self) -> OpcodeClass {
        todo!()
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
