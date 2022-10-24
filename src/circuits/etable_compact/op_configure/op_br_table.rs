use super::*;
use crate::circuits::{mtable_compact::encode::MemoryTableLookupEncode, utils::Context};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::encode::{opcode::encode_br_table, table::encode_br_table_entry};
use specs::mtable::VarType;
use specs::step::StepInfo;
use specs::{etable::EventTableEntry, itable::OpcodeClass};

pub struct BrTableConfig {
    keep: BitCell,
    keep_value: U64Cell,
    keep_type: CommonRangeCell,
    drop: CommonRangeCell,
    dst_iid: CommonRangeCell,
    expect_index: CommonRangeCell,
    effective_index: CommonRangeCell,
    targets_len: CommonRangeCell,
    out_of_bound: BitCell,
    br_table_lookup: BrTableLookupCell,
    diff: U64Cell,
    lookup_stack_read_index: MTableLookupCell,
    lookup_stack_read_return_value: MTableLookupCell,
    lookup_stack_write_return_value: MTableLookupCell,
}

pub struct BrTableConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for BrTableConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let keep = common.alloc_bit_value();
        let keep_value = common.alloc_u64();
        let keep_type = common.alloc_common_range_value();
        let drop = common.alloc_common_range_value();
        let dst_iid = common.alloc_common_range_value();

        let expect_index = common.alloc_common_range_value();
        let effective_index = common.alloc_common_range_value();
        let targets_len = common.alloc_common_range_value();
        let out_of_bound = common.alloc_bit_value();
        let diff = common.alloc_u64();

        let br_table_lookup = common.alloc_brtable_lookup();
        let lookup_stack_read_index = common.alloc_mtable_lookup();
        let lookup_stack_read_return_value = common.alloc_mtable_lookup();
        let lookup_stack_write_return_value = common.alloc_mtable_lookup();

        constraint_builder.push(
            "op_brtable oob",
            Box::new(move |meta| {
                vec![
                    // Default branch when expected >= targets_len
                    out_of_bound.expr(meta)
                        * (targets_len.expr(meta) + diff.expr(meta) - expect_index.expr(meta)),
                    /*
                     * expected_index < targets_len
                     * <= expected_index + 1 <= targets_len
                     * <= expected_index + 1 + diff = targets_len, where diff >= 0
                     */
                    (constant_from!(1) - out_of_bound.expr(meta))
                        * (expect_index.expr(meta) + diff.expr(meta) + constant_from!(1)
                            - targets_len.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_brtable effective_index",
            Box::new(move |meta| {
                vec![
                    out_of_bound.expr(meta)
                        * (targets_len.expr(meta) - constant_from!(1) - effective_index.expr(meta)),
                    ((constant_from!(1) - out_of_bound.expr(meta))
                        * (expect_index.expr(meta) - effective_index.expr(meta))),
                ]
            }),
        );

        Box::new(BrTableConfig {
            keep,
            keep_value,
            keep_type,
            drop,
            dst_iid,
            expect_index,
            effective_index,
            targets_len,
            out_of_bound,
            diff,
            br_table_lookup,
            lookup_stack_read_index,
            lookup_stack_read_return_value,
            lookup_stack_write_return_value,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for BrTableConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_br_table(self.targets_len.expr(meta))
    }

    fn brtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(encode_br_table_entry(
            common_config.moid(meta),
            common_config.fid(meta),
            common_config.iid(meta),
            self.effective_index.expr(meta),
            self.drop.expr(meta),
            self.keep.expr(meta),
            self.dst_iid.expr(meta),
        ))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::BrTable {
                index,
                dst_pc,
                drop,
                keep,
                keep_values,
            } => {
                assert!(keep.len() <= 1);

                self.drop.assign(ctx, F::from(*drop as u64))?;
                self.dst_iid.assign(ctx, F::from(*dst_pc as u64))?;
                let index = *index as u32 as u64;

                self.lookup_stack_read_index.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(entry.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(entry.sp + 1),
                        BigUint::from(VarType::I32 as u16),
                        BigUint::from(index),
                    ),
                )?;

                if keep.len() > 0 {
                    let keep_type: VarType = keep[0].into();

                    self.keep.assign(ctx, true)?;
                    self.keep_value.assign(ctx, keep_values[0])?;
                    self.keep_type.assign(ctx, F::from(keep_type as u64))?;

                    self.lookup_stack_read_return_value.assign(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_read(
                            BigUint::from(entry.eid),
                            BigUint::from(2 as u64),
                            BigUint::from(entry.sp + 2),
                            BigUint::from(keep_type as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;

                    self.lookup_stack_write_return_value.assign(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_write(
                            BigUint::from(step_info.current.eid),
                            BigUint::from(3 as u64),
                            BigUint::from(step_info.current.sp + 2 + *drop as u64),
                            BigUint::from(keep_type as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;
                }

                let targets = match &entry.inst.opcode {
                    specs::itable::Opcode::BrTable { targets } => targets.clone(),
                    _ => unreachable!(),
                };
                let targets_len = targets.len() as u64;

                self.targets_len.assign(ctx, F::from(targets_len))?;

                let effective_index = if index < targets_len {
                    index
                } else {
                    targets.len() as u64 - 1
                };
                self.expect_index.assign(ctx, F::from(index))?;
                self.effective_index.assign(ctx, F::from(effective_index))?;
                self.out_of_bound.assign(ctx, index != effective_index)?;
                println!("out: {}", index != effective_index);
                self.diff.assign(
                    ctx,
                    if index < targets_len {
                        targets_len - index - 1
                    } else {
                        index - targets_len
                    },
                )?;

                self.br_table_lookup.assign(
                    ctx,
                    &encode_br_table_entry(
                        BigUint::from(step_info.current.moid),
                        BigUint::from(step_info.current.fid),
                        BigUint::from(step_info.current.iid),
                        BigUint::from(effective_index),
                        BigUint::from(*drop),
                        BigUint::from(keep.len()),
                        BigUint::from(*dst_pc),
                    ),
                )?;
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> u64 {
        match &entry.step_info {
            StepInfo::BrTable { keep, .. } => {
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

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::BrTable
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1) + constant_from!(2) * self.keep.expr(meta))
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
                constant_from!(VarType::I32 as u32 as u64),
                self.expect_index.expr(meta),
            )),
            MLookupItem::Second => Some(
                self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(2),
                        common_config.sp(meta) + constant_from!(2),
                        self.keep_type.expr(meta),
                        self.keep_value.expr(meta),
                    ),
            ),
            MLookupItem::Third => Some(
                self.keep.expr(meta)
                    * MemoryTableLookupEncode::encode_stack_write(
                        common_config.eid(meta),
                        constant_from!(3),
                        common_config.sp(meta) + constant_from!(2) + self.drop.expr(meta),
                        self.keep_type.expr(meta),
                        self.keep_value.expr(meta),
                    ),
            ),
            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1) + self.drop.expr(meta))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.dst_iid.expr(meta))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_circuit_noexternal;

    #[test]
    fn test_br_table_1() {
        let textual_repr = r#"
            (module
                (func (export "test") (result i32)
                (block
                  (block
                    (block
                      (block
                        (block
                            (br_table 3 2 1 0 4 (i32.const 0))
                            (return (i32.const 99))
                        )
                        (return (i32.const 100))
                      )
                      (return (i32.const 101))
                    )
                  (return (i32.const 102))
                  )
                (return (i32.const 103))
                )
                (i32.const 104)
                )
            )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_table_2() {
        let textual_repr = r#"
            (module
                (func (export "test") (result i32)
                (block
                    (block
                    (block
                        (block
                        (block
                            (br_table 3 2 1 0 4 (i32.const 4))
                            (return (i32.const 99))
                        )
                        (return (i32.const 100))
                        )
                        (return (i32.const 101))
                    )
                    (return (i32.const 102))
                    )
                    (return (i32.const 103))
                )
                (i32.const 104)
                )
            )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_table_oob_1() {
        let textual_repr = r#"
            (module
              (func (export "test") (result i32)
                (block
                  (block
                    (block
                      (block
                        (block
                          (br_table 3 2 1 0 4 (i32.const 5))
                          (return (i32.const 99))
                        )
                        (return (i32.const 100))
                      )
                      (return (i32.const 101))
                    )
                  (return (i32.const 102))
                  )
                (return (i32.const 103))
                )
                (i32.const 104)
                )
            )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }

    #[test]
    fn test_br_table_oob_2() {
        let textual_repr = r#"
            (module
              (func (export "test") (result i32)
                (block
                  (block
                    (block
                      (block
                        (block
                          (br_table 3 2 1 0 4 (i32.const 99))
                          (return (i32.const 99))
                        )
                        (return (i32.const 100))
                      )
                      (return (i32.const 101))
                    )
                  (return (i32.const 102))
                  )
                (return (i32.const 103))
                )
                (i32.const 104)
                )
            )
        "#;

        test_circuit_noexternal(textual_repr).unwrap();
    }
}
