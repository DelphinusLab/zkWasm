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
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{encode::opcode::encode_br_if_eqz, itable::OPCODE_ARG1_SHIFT};
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_CLASS_SHIFT},
};

pub struct BrIfEqzConfig {
    cond: U64Cell,
    cond_inv: UnlimitedCell,
    keep: BitCell,
    keep_value: U64Cell,
    keep_type: CommonRangeCell,
    drop: CommonRangeCell,
    dst_pc: CommonRangeCell,
    lookup_stack_read_cond: MTableLookupCell,
    lookup_stack_read_return_value: MTableLookupCell,
    lookup_stack_write_return_value: MTableLookupCell,
}

pub struct BrIfEqzConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrIfEqzConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let cond = common.alloc_u64();
        let cond_inv = common.alloc_unlimited_value();
        let keep = common.alloc_bit_value();
        let keep_value = common.alloc_u64();
        let keep_type = common.alloc_common_range_value();
        let drop = common.alloc_common_range_value();
        let dst_pc = common.alloc_common_range_value();
        let lookup_stack_read_cond = common.alloc_mtable_lookup();
        let lookup_stack_read_return_value = common.alloc_mtable_lookup();
        let lookup_stack_write_return_value = common.alloc_mtable_lookup();

        // TODO: add constraints for br_if_eqz and br_if

        Box::new(BrIfEqzConfig {
            cond,
            cond_inv,
            keep,
            keep_value,
            keep_type,
            drop,
            dst_pc,
            lookup_stack_read_cond,
            lookup_stack_read_return_value,
            lookup_stack_write_return_value,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrIfEqzConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_br_if_eqz(
            self.drop.expr(meta),
            self.keep.expr(meta),
            self.dst_pc.expr(meta),
        )
    }

    fn assign(
        &self,
        _ctx: &mut Context<'_, F>,
        _step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::BrIfEqz { .. } => {
                // TODO
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BrIfEqz
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        // FIXME: fill correct value
        Some(constant_from!(0))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            // TODO
            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        // FIXME: fill correct value
        None
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        // FIXME: fill correct value
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_br_if_eqz_trivial_nojump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (i32.const 0)
                br_if_eqz 1
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_eqz_trivial_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (i32.const 0)
                br_if_eqz 0
                (i32.const 0)
                drop
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_eqz_block_with_arg_do_not_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block (result i32)
                (i32.const 0)
                (i32.const 0)
                br_if_eqz 1
              )
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_eqz_block_with_arg_do_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block (result i32)
                (i32.const 0)
                (i32.const 1)
                br_if_eqz 0
              )
              drop
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_eqz_block_with_drop_do_not_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (block
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 0)
                  br_if_eqz 1
                  drop
                  drop
                )
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_if_eqz_block_with_drop_do_jump_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (block
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 1)
                  br_if_eqz 0
                  drop
                  drop
                )
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
