use super::*;
use crate::{
    circuits::utils::{bn_to_field, Context},
    constant, constant_from, curr, next,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
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
        meta: &mut ConstraintSystem<F>,
        common: &mut EventTableCellAllocator,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
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

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        todo!()
    }

    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error> {
        todo!()
    }

    fn opcode_class(&self) -> OpcodeClass {
        todo!()
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }

    fn last_jump_eid_change(&self) -> Option<Expression<F>> {
        None
    }

    fn next_iid(&self) -> Option<Expression<F>> {
        None
    }

    fn next_moid(&self) -> Option<Expression<F>> {
        None
    }

    fn mtable_lookup(&self, i: i32) -> Option<Expression<F>> {
        None
    }

    fn jtable_lookup(&self) -> Option<Expression<F>> {
        None
    }

    fn itable_lookup(&self) -> Option<Expression<F>> {
        None
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
