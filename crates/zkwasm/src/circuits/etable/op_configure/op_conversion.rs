use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::opcode::encode_conversion;
use specs::etable::EventTableEntry;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct ConversionConfig<F: FieldExt> {
    value: AllocatedU64Cell<F>,
    value_is_i8: AllocatedBitCell<F>,
    value_is_i16: AllocatedBitCell<F>,
    value_is_i32: AllocatedBitCell<F>,
    value_is_i64: AllocatedBitCell<F>,
    value_type_is_i32: AllocatedBitCell<F>,
    res_is_i32: AllocatedBitCell<F>,
    res_is_i64: AllocatedBitCell<F>,

    sign_op: AllocatedBitCell<F>,
    is_i32_wrap_i64: AllocatedBitCell<F>,

    // Sign-extension proposal
    flag_bit: AllocatedBitCell<F>,
    rem: AllocatedU64Cell<F>,
    shift: AllocatedUnlimitedCell<F>,
    padding: AllocatedUnlimitedCell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,
}

pub struct ConversionConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for ConversionConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let value = allocator.alloc_u64_cell();

        let value_is_i8 = allocator.alloc_bit_cell();
        let value_is_i16 = allocator.alloc_bit_cell();
        let value_is_i32 = allocator.alloc_bit_cell();
        let value_is_i64 = allocator.alloc_bit_cell();

        let value_type_is_i32 = allocator.alloc_bit_cell();

        let res_is_i32 = allocator.alloc_bit_cell();
        let res_is_i64 = allocator.alloc_bit_cell();

        let sign_op = allocator.alloc_bit_cell();
        let is_i32_wrap_i64 = allocator.alloc_bit_cell();

        let flag_bit = allocator.alloc_bit_cell();
        let rem = allocator.alloc_u64_cell();
        let shift = allocator.alloc_unlimited_cell();
        let padding = allocator.alloc_unlimited_cell();

        let eid = common_config.eid_cell;
        let sp = common_config.sp_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "op_conversion stack read",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| value_type_is_i32.expr(meta),
            move |meta| value.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator
            .alloc_memory_table_lookup_write_cell_with_value(
                "op_conversion stack write",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(1),
                move |meta| res_is_i32.expr(meta),
                move |____| constant_from!(1),
            );

        let res = memory_table_lookup_stack_write.value_cell;

        /*
         * Implicit Constraint:
         *
         * value_is_i8 || value_is_i16 || value_is_i32 || value_is_i64 can be constrained by opcode.
         * res_is_i32  || res_is_i64 can be constrained by opcde.
         */

        constraint_builder.push(
            "op_conversion i32_wrap_i64",
            Box::new(move |meta| {
                vec![
                    is_i32_wrap_i64.expr(meta)
                        * (value_is_i64.expr(meta) + res_is_i32.expr(meta) - constant_from!(2)),
                    is_i32_wrap_i64.expr(meta)
                        * (value.u16_cells_le[1].expr(meta) * constant_from!(1 << 16)
                            + value.u16_cells_le[0].expr(meta)
                            - res.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_conversion helper",
            Box::new(move |meta| {
                vec![
                    value_is_i8.expr(meta) * (constant_from!(1u64 << 7) - shift.expr(meta))
                        + value_is_i8.expr(meta) * (constant_from!(1u64 << 7) - shift.expr(meta))
                        + value_is_i16.expr(meta) * (constant_from!(1u64 << 15) - shift.expr(meta))
                        + value_is_i16.expr(meta) * (constant_from!(1u64 << 15) - shift.expr(meta))
                        + value_is_i32.expr(meta) * (constant_from!(1u64 << 31) - shift.expr(meta)),
                    value_is_i8.expr(meta)
                        * res_is_i32.expr(meta)
                        * (constant_from!((u32::MAX << 8) as u64) - padding.expr(meta))
                        + value_is_i16.expr(meta)
                            * res_is_i32.expr(meta)
                            * (constant_from!((u32::MAX << 16) as u64) - padding.expr(meta))
                        + value_is_i8.expr(meta)
                            * res_is_i64.expr(meta)
                            * (constant_from!(u64::MAX << 8) - padding.expr(meta))
                        + value_is_i16.expr(meta)
                            * res_is_i64.expr(meta)
                            * (constant_from!(u64::MAX << 16) - padding.expr(meta))
                        + value_is_i32.expr(meta)
                            * res_is_i64.expr(meta)
                            * (constant_from!(u64::MAX << 32) - padding.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_conversion: sign extension",
            Box::new(move |meta| {
                vec![
                    // Unsigned extension: i64.extend_i32u
                    (sign_op.expr(meta) - constant_from!(1))
                        * (is_i32_wrap_i64.expr(meta) - constant_from!(1))
                        * (res.expr(meta) - value.expr(meta)),
                    // Split Operand for all ext-sign instructions and is_i64_extend_i32_s,
                    // though other instructions still met it by meaningless assignment.
                    flag_bit.expr(meta) * shift.expr(meta) + rem.expr(meta) - value.expr(meta),
                    // Compose Result for all ext-sign instructions and is_i64_extend_i32_s.
                    sign_op.expr(meta)
                        * (flag_bit.expr(meta) * padding.expr(meta) + value.expr(meta)
                            - res.expr(meta)),
                ]
            }),
        );

        Box::new(ConversionConfig {
            value,
            value_is_i8,
            value_is_i16,
            value_is_i32,
            value_is_i64,
            value_type_is_i32,
            res_is_i32,
            res_is_i64,
            sign_op,
            is_i32_wrap_i64,
            flag_bit,
            rem,
            shift,
            padding,
            memory_table_lookup_stack_read,
            memory_table_lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ConversionConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_conversion::<Expression<F>>(
            self.sign_op.expr(meta),
            self.value_type_is_i32.expr(meta),
            self.value_is_i8.expr(meta),
            self.value_is_i16.expr(meta),
            self.value_is_i32.expr(meta),
            self.value_is_i64.expr(meta),
            self.res_is_i32.expr(meta),
            self.res_is_i64.expr(meta),
        )
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        let (value, value_type, result, result_type) = match &entry.eentry.step_info {
            StepInfo::I32WrapI64 { value, result } => {
                self.value_is_i64.assign_bool(ctx, true)?;
                self.res_is_i32.assign_bool(ctx, true)?;
                self.rem.assign(ctx, *value as u64)?;
                self.is_i32_wrap_i64.assign_bool(ctx, true)?;

                (
                    *value as u64,
                    VarType::I64,
                    *result as u32 as u64,
                    VarType::I32,
                )
            }
            StepInfo::I64ExtendI32 {
                value,
                result,
                sign,
            } => {
                self.value_is_i32.assign_bool(ctx, true)?;
                self.res_is_i64.assign_bool(ctx, true)?;
                self.sign_op.assign_bool(ctx, *sign)?;
                self.flag_bit.assign_bool(ctx, *value >> 31 != 0)?;
                self.rem.assign(ctx, (*value & 0x7fffffff) as u64)?;
                self.shift.assign(ctx, F::from(1 << 31))?;
                self.padding.assign(ctx, F::from((u64::MAX << 32) as u64))?;

                (
                    *value as u32 as u64,
                    VarType::I32,
                    *result as u64,
                    VarType::I64,
                )
            }
            StepInfo::I32SignExtendI8 { value, result } => {
                self.value_is_i8.assign_bool(ctx, true)?;
                self.res_is_i32.assign_bool(ctx, true)?;
                self.sign_op.assign_bool(ctx, true)?;
                self.flag_bit.assign_bool(ctx, *value >> 7 != 0)?;
                self.rem.assign(ctx, (*value & 0x7f) as u64)?;
                self.shift.assign(ctx, F::from(1 << 7))?;
                self.padding.assign(ctx, F::from((u32::MAX << 8) as u64))?;

                (
                    *value as u32 as u64,
                    VarType::I32,
                    *result as u32 as u64,
                    VarType::I32,
                )
            }
            StepInfo::I32SignExtendI16 { value, result } => {
                self.value_is_i16.assign_bool(ctx, true)?;
                self.res_is_i32.assign_bool(ctx, true)?;
                self.sign_op.assign_bool(ctx, true)?;
                self.flag_bit.assign_bool(ctx, *value >> 15 != 0)?;
                self.rem.assign(ctx, (*value & 0x7fff) as u64)?;
                self.shift.assign(ctx, F::from(1 << 15))?;
                self.padding.assign(ctx, F::from((u32::MAX << 16) as u64))?;

                (
                    *value as u32 as u64,
                    VarType::I32,
                    *result as u32 as u64,
                    VarType::I32,
                )
            }
            StepInfo::I64SignExtendI8 { value, result } => {
                self.value_is_i8.assign_bool(ctx, true)?;
                self.res_is_i64.assign_bool(ctx, true)?;
                self.sign_op.assign_bool(ctx, true)?;
                self.flag_bit.assign_bool(ctx, *value >> 7 != 0)?;
                self.rem.assign(ctx, (*value & 0x7f) as u64)?;
                self.shift.assign(ctx, F::from(1 << 7))?;
                self.padding.assign(ctx, F::from((u64::MAX << 8) as u64))?;

                (*value as u64, VarType::I64, *result as u64, VarType::I64)
            }
            StepInfo::I64SignExtendI16 { value, result } => {
                self.value_is_i16.assign_bool(ctx, true)?;
                self.res_is_i64.assign_bool(ctx, true)?;
                self.sign_op.assign_bool(ctx, true)?;
                self.flag_bit.assign_bool(ctx, *value >> 15 != 0)?;
                self.rem.assign(ctx, (*value & 0x7fff) as u64)?;
                self.shift.assign(ctx, F::from(1 << 15))?;
                self.padding.assign(ctx, F::from((u64::MAX << 16) as u64))?;

                (*value as u64, VarType::I64, *result as u64, VarType::I64)
            }
            StepInfo::I64SignExtendI32 { value, result } => {
                self.value_is_i32.assign_bool(ctx, true)?;
                self.res_is_i64.assign_bool(ctx, true)?;
                self.sign_op.assign_bool(ctx, true)?;
                self.flag_bit.assign_bool(ctx, *value >> 31 != 0)?;
                self.rem.assign(ctx, (*value & 0x7fffffff) as u64)?;
                self.shift.assign(ctx, F::from(1 << 31))?;
                self.padding.assign(ctx, F::from((u64::MAX << 32) as u64))?;

                (*value as u64, VarType::I64, *result as u64, VarType::I64)
            }
            _ => unreachable!(),
        };

        self.value.assign(ctx, value)?;
        self.value_type_is_i32
            .assign(ctx, F::from(value_type as u64))?;
        self.res_is_i32.assign(ctx, F::from(result_type as u64))?;

        self.memory_table_lookup_stack_read.assign(
            ctx,
            entry.memory_rw_entires[0].start_eid,
            step.current.eid,
            entry.memory_rw_entires[0].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            value_type == VarType::I32,
            value,
        )?;

        self.memory_table_lookup_stack_write.assign(
            ctx,
            step.current.eid,
            entry.memory_rw_entires[1].end_eid,
            step.current.sp + 1,
            LocationType::Stack,
            result_type == VarType::I32,
            result,
        )?;

        Ok(())
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }
}
