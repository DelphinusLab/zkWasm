use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::{BigUint, ToBigUint};
use specs::{
    encode::frame_table::encode_frame_table_entry,
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::VarType,
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
        utils::{bn_to_field, Context},
    },
    constant, constant_from,
};

pub struct ReturnConfig<F: FieldExt> {
    keep: AllocatedBitCell<F>,
    drop: AllocatedCommonRangeCell<F>,
    vtype: AllocatedCommonRangeCell<F>,
    value: AllocatedU64Cell<F>,
    frame_table_lookup: AllocatedUnlimitedCell<F>,
    memory_table_lookup_stack_read: AllocatedMemoryTableLookupCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupCell<F>,
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

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_cell();
        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_cell();
        let frame_table_lookup = common_config.jtable_lookup_cell;

        let fid_cell = common_config.fid_cell;
        let iid_cell = common_config.iid_cell;
        let frame_id_cell = common_config.frame_id_cell;
        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        constraint_builder.constraints.push((
            "return frame table lookups",
            Box::new(move |meta| {
                vec![
                    frame_table_lookup.expr(meta)
                        - JumpTableConfig::encode_lookup(
                            frame_id_cell.expr(meta),
                            frame_id_cell.next_expr(meta),
                            fid_cell.expr(meta),
                            fid_cell.next_expr(meta),
                            iid_cell.next_expr(meta),
                        ),
                ]
            }),
        ));

        constraint_builder.constraints.push((
            "return memory table lookups",
            Box::new(move |meta| {
                vec![
                    memory_table_lookup_stack_read.expr(meta)
                        - keep.expr(meta)
                            * MemoryTableLookupEncode::encode_stack_read(
                                eid.expr(meta),
                                constant_from!(1),
                                sp.expr(meta) + constant_from!(1),
                                vtype.expr(meta),
                                value.u64_cell.expr(meta),
                            ),
                    memory_table_lookup_stack_write.expr(meta)
                        - keep.expr(meta)
                            * MemoryTableLookupEncode::encode_stack_write(
                                eid.expr(meta),
                                constant_from!(2),
                                sp.expr(meta) + drop.expr(meta) + constant_from!(1),
                                vtype.expr(meta),
                                value.u64_cell.expr(meta),
                            ),
                ]
            }),
        ));

        Box::new(ReturnConfig {
            keep,
            drop,
            vtype,
            value,
            frame_table_lookup,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ReturnConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Return as u64) << OPCODE_CLASS_SHIFT)
        )) + self.drop.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + self.keep.expr(meta)
                * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.vtype.expr(meta)
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

                    self.memory_table_lookup_stack_read.assign_bn(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_read(
                            BigUint::from(entry.eid),
                            BigUint::from(1 as u64),
                            BigUint::from(entry.sp + 1),
                            BigUint::from(vtype as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;
                    self.memory_table_lookup_stack_write.assign_bn(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_write(
                            BigUint::from(entry.eid),
                            BigUint::from(2 as u64),
                            BigUint::from(entry.sp + *drop + 1),
                            BigUint::from(vtype as u16),
                            BigUint::from(keep_values[0]),
                        ),
                    )?;
                }
                self.frame_table_lookup.assign_bn(
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

    fn next_frame_id(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.frame_id_cell.next_expr(meta))
    }

    fn next_fid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.fid_cell.next_expr(meta))
    }

    fn next_iid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.iid_cell.next_expr(meta))
    }
}
