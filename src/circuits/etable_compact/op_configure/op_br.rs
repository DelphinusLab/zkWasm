use super::*;
use crate::circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::step::StepInfo;
use specs::{encode::opcode::encode_br, mtable::VarType};
use specs::{etable::EventTableEntry, itable::OpcodeClass};

pub struct BrConfig {
    keep: BitCell,
    keep_value: U64Cell,
    keep_type: CommonRangeCell,
    drop: CommonRangeCell,
    dst_pc: CommonRangeCell,
    lookup_stack_read_return_value: MTableLookupCell,
    lookup_stack_write_return_value: MTableLookupCell,
}

pub struct BrConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let keep = common.alloc_bit_value();
        let keep_value = common.alloc_u64();
        let keep_type = common.alloc_common_range_value();
        let drop = common.alloc_common_range_value();
        let dst_pc = common.alloc_common_range_value();
        let lookup_stack_read_return_value = common.alloc_mtable_lookup();
        let lookup_stack_write_return_value = common.alloc_mtable_lookup();

        Box::new(BrConfig {
            keep,
            keep_value,
            keep_type,
            drop,
            dst_pc,
            lookup_stack_read_return_value,
            lookup_stack_write_return_value,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_br(
            self.drop.expr(meta),
            self.keep.expr(meta),
            self.dst_pc.expr(meta),
        )
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::Br {
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                let drop: u16 = (*drop).try_into().unwrap();

                self.drop.assign(ctx, F::from(drop as u64))?;

                if keep.len() > 0 {
                    let keep_type: VarType = keep[0].into();

                    self.keep.assign(ctx, true)?;
                    self.keep_value.assign(ctx, keep_values[0])?;
                    self.keep_type.assign(ctx, F::from(keep_type as u64))?;

                    self.lookup_stack_read_return_value.assign(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_read(
                            BigUint::from(entry.eid),
                            BigUint::from(1 as u64),
                            BigUint::from(entry.sp + 1),
                            BigUint::from(keep_type as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;

                    self.lookup_stack_write_return_value.assign(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_write(
                            BigUint::from(step_info.current.eid),
                            BigUint::from(2 as u64),
                            BigUint::from(step_info.current.sp + 1 + drop as u64),
                            BigUint::from(keep_type as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;
                }

                self.dst_pc.assign(ctx, F::from((*dst_pc) as u64))?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Br
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2) * self.keep.expr(meta))
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> u64 {
        match &entry.step_info {
            StepInfo::Br { keep, .. } => {
                if keep.len() > 0 {
                    assert!(keep.len() == 1);
                    2
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(
                self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(1),
                        common_config.sp(meta) + constant_from!(1),
                        self.keep_type.expr(meta),
                        self.keep_value.expr(meta),
                    ),
            ),

            MLookupItem::Second => Some(
                self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_write(
                        common_config.eid(meta),
                        constant_from!(2),
                        common_config.sp(meta) + constant_from!(1) + self.drop.expr(meta),
                        self.keep_type.expr(meta),
                        self.keep_value.expr(meta),
                    ),
            ),

            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.drop.expr(meta))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.dst_pc.expr(meta))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_br_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                br 0
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_drop_ok() {
        let textual_repr = r#"
        (module
            (func (export "test")
              (block
                (i32.const 0)
                br 0
              )
            )
           )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
