use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::{BigUint, ToBigUint};
use specs::{
    encode::table::encode_frame_table_entry, etable::EventTableEntry, mtable::VarType,
    step::StepInfo,
};

use crate::{
    circuits::{
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder, StepStatus,
        },
        jtable::{expression::JtableLookupEntryEncode, JumpTableConfig},
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::Context,
    },
    constant_from,
};

pub struct ReturnConfig<F: FieldExt> {
    keep: AllocatedBitCell<F>,
    drop: AllocatedCommonRangeCell<F>,
    vtype: AllocatedCommonRangeCell<F>,
    value: AllocatedU64Cell<F>,
    jtable_lookup: AllocatedUnlimitedCell<F>,
    mtable_lookup_stack_read: AllocatedMemoryTableLookupCell<F>,
    mtable_lookup_stack_write: AllocatedMemoryTableLookupCell<F>,
}

pub struct ReturnConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ReturnConfigBuilder {
    fn configure(
        common_config: &mut EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let keep = allocator.alloc_bit_cell();
        let drop = allocator.alloc_common_range_cell();
        let vtype = allocator.alloc_common_range_cell();
        let value = allocator.alloc_u64_cell();

        let mtable_lookup_stack_read = allocator.alloc_memory_table_lookup_cell();
        let mtable_lookup_stack_write = allocator.alloc_memory_table_lookup_cell();
        let jtable_lookup = common_config.jtable_lookup_cell;

        /*
            I want to remove constraint_builder...
            constraint_builder.constraints.push((
                "return jtable lookups",
                Box::new(move |meta| {
                    vec![
                        jtable_lookup.expr(meta)
                            - JumpTableConfig::encode_lookup(
                                common_config.frame_id_cell.expr(meta),
                                common_config.frame_id_cell.next_expr(meta),
                                common_config.fid_cell.expr(meta),
                                common_config.fid_cell.next_expr(meta),
                                common_config.iid_cell.next_expr(meta),
                            ),
                    ]
                }),
            ));

            constraint_builder.constraints.push((
                "return mtable lookups",
                Box::new(move |meta| {
                    vec![
                        mtable_lookup_stack_read.expr(meta)
                            - self.keep.expr(meta)
                            * MemoryTableLookupEncode::encode_stack_read(
                                common.eid(meta),
                                constant_from!(1),
                                common.sp(meta) + constant_from!(1),
                                self.vtype.expr(meta),
                                self.value.expr(meta),
                            ),

                        mtable_lookup_stack_write.expr(meta)
                            - self.keep.expr(meta)
                            * MemoryTableLookupEncode::encode_stack_write(
                                common.eid(meta),
                                constant_from!(2),
                                common.sp(meta) + self.drop.expr(meta) + constant_from!(1),
                                self.vtype.expr(meta),
                                self.value.expr(meta),
                            ),
                    ]
                }),
            ));
        */

        Box::new(ReturnConfig {
            keep,
            drop,
            vtype,
            value,
            jtable_lookup,
            mtable_lookup_stack_read,
            mtable_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ReturnConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        todo!()
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::Return {
                drop,
                keep,
                keep_values,
                ..
            } => {
                assert!(keep.len() <= 1);
                assert!(*drop < 1 << 16);
                assert_eq!(keep.len(), keep_values.len());

                self.drop.assign(ctx, F::from(*drop as u64))?;

                if keep_values.len() == 0 {
                    self.keep.assign(ctx, 0.into())?;
                } else {
                    self.keep.assign(ctx, 1.into())?;
                    let vtype = VarType::from(keep[0]);
                    self.vtype.assign(ctx, (vtype as u64).into())?;
                    self.value.assign(ctx, keep_values[0])?;

                    self.mtable_lookup_stack_read.assign_bn(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_read(
                            BigUint::from(entry.eid),
                            BigUint::from(1 as u64),
                            BigUint::from(entry.sp + 1),
                            BigUint::from(vtype as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;
                    self.mtable_lookup_stack_write.assign_bn(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_write(
                            BigUint::from(entry.eid),
                            BigUint::from(2 as u64),
                            BigUint::from(entry.sp + *drop as u64 + 1),
                            BigUint::from(vtype as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;
                }
                self.jtable_lookup.assign_bn(
                    ctx,
                    &encode_frame_table_entry(
                        step.current.last_jump_eid.to_biguint().unwrap(),
                        step.next.last_jump_eid.to_biguint().unwrap(),
                        step.current.fid.to_biguint().unwrap(),
                        step.next.fid.to_biguint().unwrap(),
                        step.next.iid.to_biguint().unwrap(),
                    ),
                )?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(self.drop.expr(meta))
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
            StepInfo::Return { keep, .. } => {
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

    fn jops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }
}
