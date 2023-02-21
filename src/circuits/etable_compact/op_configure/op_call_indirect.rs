use super::*;
use crate::{
    circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context},
    constant,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::ToBigUint;
use specs::{encode::opcode::encode_call_indirect, mtable::VarType, step::StepInfo};
use specs::{
    encode::{br_table::encode_elem_entry, frame_table::encode_frame_table_entry},
    etable::EventTableEntry,
};

pub struct CallIndirectConfig {
    type_index: CommonRangeCell,
    func_index: CommonRangeCell,
    offset: CommonRangeCell,
    table_index: CommonRangeCell,
    stack_read_lookup: MTableLookupCell,
    elem_lookup: BrTableLookupCell,
    frame_table_lookup: JTableLookupCell,
}

pub struct CallIndirectConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallIndirectConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let type_index = common.alloc_common_range_value();
        let table_index = common.alloc_common_range_value();
        let offset = common.alloc_common_range_value();
        let func_index = common.alloc_common_range_value();

        let elem_lookup = common.alloc_brtable_lookup();
        let stack_read_lookup = common.alloc_mtable_lookup();
        let frame_table_lookup = common.alloc_jtable_lookup();

        // Wasmi only support one table.
        constraint_builder.push(
            "table_index",
            Box::new(move |meta| vec![table_index.expr(meta)]),
        );

        Box::new(CallIndirectConfig {
            type_index,
            func_index,
            offset,
            table_index,
            elem_lookup,
            stack_read_lookup,
            frame_table_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallIndirectConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call_indirect(self.type_index.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::CallIndirect {
                table_index,
                type_index,
                offset,
                func_index,
            } => {
                self.table_index.assign(ctx, F::from(*table_index as u64))?;
                self.type_index.assign(ctx, F::from(*type_index as u64))?;
                self.offset.assign(ctx, F::from(*offset as u64))?;
                self.func_index.assign(ctx, F::from(*func_index as u64))?;

                self.elem_lookup.assign(
                    ctx,
                    &encode_elem_entry(
                        BigUint::from(*table_index),
                        BigUint::from(*type_index),
                        BigUint::from(*offset),
                        BigUint::from(*func_index),
                    ),
                )?;

                self.stack_read_lookup.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(*offset),
                    ),
                )?;

                self.frame_table_lookup.assign(
                    ctx,
                    &encode_frame_table_entry(
                        step_info.current.eid.to_biguint().unwrap(),
                        step_info.current.last_jump_eid.to_biguint().unwrap(),
                        (*func_index).to_biguint().unwrap(),
                        step_info.current.fid.to_biguint().unwrap(),
                        (step_info.current.iid + 1).to_biguint().unwrap(),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant!(F::one()))
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
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32),
                self.offset.expr(meta),
            )),
            _ => None,
        }
    }

    fn brtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(encode_elem_entry(
            self.table_index.expr(meta),
            self.type_index.expr(meta),
            self.offset.expr(meta),
            self.func_index.expr(meta),
        ))
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
        Some(self.func_index.expr(meta))
    }

    fn next_iid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(constant_from!(0))
    }

    fn jtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(encode_frame_table_entry(
            common_config.eid(meta),
            common_config.last_jump_eid(meta),
            self.func_index.expr(meta),
            common_config.fid(meta),
            common_config.iid(meta) + constant_from!(1),
        ))
    }
}

#[cfg(test)]

mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_call_indirect() {
        let textual_repr = r#"
            (module
                (type (;0;) (func (param i32 i32) (result i32)))
                (func (;0;) (type 0) (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add)
                (func (;1;) (type 0) (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.sub)
                (func (;2;) (result i32)
                    i32.const 1
                    i32.const 2
                    i32.const 1
                    call_indirect (type 0))
                (table (;0;) 2 2 funcref)
                (export "test" (func 2))
                (elem (;0;) (i32.const 0) func 0 1)
            )
        "#;

        test_circuit_noexternal(textual_repr).unwrap()
    }
}
