use std::marker::PhantomData;

use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        jtable::{expression::JtableLookupEntryEncode, JumpTableConfig},
        utils::{
            bn_to_field, step_status::StepStatus, table_entry::EventTableEntryWithMemoryInfo,
            Context,
        },
    },
    constant, constant_from, constant_from_bn,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::{BigUint, ToBigUint};
use specs::{
    configure_table::WASM_PAGE_SIZE,
    encode::{frame_table::encode_frame_table_entry, memory_table, FromBn},
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct LoadConfig<F: FieldExt> {
    // offset in opcode
    opcode_load_offset: AllocatedCommonRangeCell<F>,

    value_leading_u16: AllocatedUnlimitedCell<F>,
    value_leading_u16_u8_high: AllocatedCommonRangeCell<F>,
    value_leading_u16_u8_low: AllocatedCommonRangeCell<F>,
    value_leading_u16_u8_low_diff: AllocatedCommonRangeCell<F>,

    value_flag: AllocatedBitCell<F>,
    value_parsed_leading_u8_rem: AllocatedCommonRangeCell<F>,
    value_parsed_leading_u8_rem_diff: AllocatedCommonRangeCell<F>,

    // which heap offset to load
    load_block_index: AllocatedU64Cell<F>,
    load_inner_pos: [AllocatedBitCell<F>; 3],

    load_value_in_heap: AllocatedU64Cell<F>,
    // value after pick bytes from load
    load_value_parsed: AllocatedU64Cell<F>,
    // value after extension
    res: AllocatedUnlimitedCell<F>,

    // load offset arg
    load_base: AllocatedU64Cell<F>,

    is_one_byte: AllocatedBitCell<F>,
    is_two_bytes: AllocatedBitCell<F>,
    is_four_bytes: AllocatedBitCell<F>,
    is_eight_bytes: AllocatedBitCell<F>,
    is_sign: AllocatedBitCell<F>,
    is_i32: AllocatedBitCell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,

    address_within_allocated_pages_helper: AllocatedCommonRangeCell<F>,
}

pub struct LoadConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LoadConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let opcode_load_offset = allocator.alloc_common_range_cell();
        let load_base = allocator.alloc_u64_cell();

        let value_leading_u16 = allocator.alloc_unlimited_cell();
        let value_leading_u16_u8_high = allocator.alloc_common_range_cell();
        let value_leading_u16_u8_low = allocator.alloc_common_range_cell();
        let value_leading_u16_u8_low_diff = allocator.alloc_common_range_cell();

        let value_flag = allocator.alloc_bit_cell();
        let value_parsed_leading_u8_rem = allocator.alloc_common_range_cell();
        let value_parsed_leading_u8_rem_diff = allocator.alloc_common_range_cell();

        // which heap offset to load
        let load_block_index = allocator.alloc_u64_cell();
        let load_inner_pos = [0; 3].map(|x| allocator.alloc_bit_cell());

        let load_value_in_heap = allocator.alloc_u64_cell();
        // value after pick bytes from load
        let load_value_parsed = allocator.alloc_u64_cell();

        // value after extension
        let res = allocator.alloc_unlimited_cell();

        let is_one_byte = allocator.alloc_bit_cell();
        let is_two_bytes = allocator.alloc_bit_cell();
        let is_four_bytes = allocator.alloc_bit_cell();
        let is_eight_bytes = allocator.alloc_bit_cell();
        let is_sign = allocator.alloc_bit_cell();
        let is_i32 = allocator.alloc_bit_cell();

        constraint_builder.push(
            "op_load length",
            Box::new(move |meta| {
                vec![
                    is_one_byte.expr(meta)
                        + is_two_bytes.expr(meta)
                        + is_four_bytes.expr(meta)
                        + is_eight_bytes.expr(meta)
                        - constant_from!(1),
                ]
            }),
        );

        constraint_builder.push(
            "op_load load target",
            Box::new(move |meta| {
                vec![
                    (load_block_index.expr(meta) * constant_from!(8)
                        + load_inner_pos[2].expr(meta) * constant_from!(4)
                        + load_inner_pos[1].expr(meta) * constant_from!(2)
                        + load_inner_pos[0].expr(meta))
                        - opcode_load_offset.expr(meta)
                        - load_base.expr(meta),
                ]
            }),
        );

        // We only support load aligned in this version
        constraint_builder.push(
            "op_load load align",
            Box::new(move |meta| {
                vec![
                    is_eight_bytes.expr(meta)
                        * (load_inner_pos[2].expr(meta)
                            + load_inner_pos[1].expr(meta)
                            + load_inner_pos[0].expr(meta)),
                    is_four_bytes.expr(meta)
                        * (load_inner_pos[1].expr(meta) + load_inner_pos[0].expr(meta)),
                    is_two_bytes.expr(meta) * (load_inner_pos[0].expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick value",
            Box::new(move |meta| {
                vec![
                    // ask for a better encoding
                    value_leading_u16.expr(meta)
                        - (is_two_bytes.expr(meta) + is_one_byte.expr(meta))
                            * ((load_inner_pos[2].expr(meta)
                                * load_inner_pos[1].expr(meta)
                                * load_value_in_heap.u16_cells_le[3].expr(meta))
                                + (constant_from!(1) - load_inner_pos[2].expr(meta))
                                    * load_inner_pos[1].expr(meta)
                                    * (load_value_in_heap.u16_cells_le[1].expr(meta))
                                + load_inner_pos[2].expr(meta)
                                    * (constant_from!(1) - load_inner_pos[1].expr(meta))
                                    * (load_value_in_heap.u16_cells_le[2].expr(meta))
                                + (constant_from!(1) - load_inner_pos[2].expr(meta))
                                    * (constant_from!(1) - load_inner_pos[1].expr(meta))
                                    * (load_value_in_heap.u16_cells_le[0].expr(meta)))
                        - is_four_bytes.expr(meta)
                            * (load_inner_pos[2].expr(meta)
                                * load_value_in_heap.u16_cells_le[3].expr(meta)
                                + (constant_from!(1) - load_inner_pos[2].expr(meta))
                                    * load_value_in_heap.u16_cells_le[1].expr(meta))
                        - is_eight_bytes.expr(meta) * load_value_in_heap.u16_cells_le[3].expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick u16 decompose",
            Box::new(move |meta| {
                vec![
                    value_leading_u16_u8_high.expr(meta) * constant_from!(1 << 8)
                        + value_leading_u16_u8_low.expr(meta)
                        - value_leading_u16.expr(meta),
                    value_leading_u16_u8_low.expr(meta) + value_leading_u16_u8_low_diff.expr(meta)
                        - constant_from!(255),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick for 2 byte & 1 byte",
            Box::new(move |meta| {
                vec![
                    load_value_parsed.expr(meta)
                        - is_eight_bytes.expr(meta) * load_value_in_heap.expr(meta)
                        - is_four_bytes.expr(meta)
                            * load_inner_pos[2].expr(meta)
                            * (load_value_in_heap.u16_cells_le[2].expr(meta)
                                + load_value_in_heap.u16_cells_le[3].expr(meta)
                                    * constant_from!(1 << 16))
                        - is_four_bytes.expr(meta)
                            * (constant_from!(1) - load_inner_pos[2].expr(meta))
                            * (load_value_in_heap.u16_cells_le[0].expr(meta)
                                + load_value_in_heap.u16_cells_le[1].expr(meta)
                                    * constant_from!(1 << 16))
                        - is_two_bytes.expr(meta) * (value_leading_u16.expr(meta))
                        - is_one_byte.expr(meta)
                            * load_inner_pos[0].expr(meta)
                            * (value_leading_u16_u8_high.expr(meta))
                        - is_one_byte.expr(meta)
                            * (constant_from!(1) - load_inner_pos[0].expr(meta))
                            * (value_leading_u16_u8_low.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_load flag",
            Box::new(move |meta| {
                let value_leading_u8 = is_one_byte.expr(meta)
                    * (load_inner_pos[0].expr(meta) * value_leading_u16_u8_high.expr(meta)
                        + (constant_from!(1) - load_inner_pos[0].expr(meta))
                            * value_leading_u16_u8_low.expr(meta))
                    + (constant_from!(1) - is_one_byte.expr(meta))
                        * value_leading_u16_u8_high.expr(meta);
                vec![
                    value_flag.expr(meta) * constant_from!(128)
                        + value_parsed_leading_u8_rem.expr(meta)
                        - value_leading_u8,
                    value_parsed_leading_u8_rem.expr(meta)
                        + value_parsed_leading_u8_rem_diff.expr(meta)
                        - constant_from!(127),
                ]
            }),
        );

        constraint_builder.push(
            "op_load extension",
            Box::new(move |meta| {
                vec![
                    is_sign.expr(meta)
                        * value_flag.expr(meta)
                        * (is_one_byte.expr(meta) * constant_from!(0xffffff00)
                            + is_two_bytes.expr(meta) * constant_from!(0xffff0000)
                            + (constant_from!(1) - is_eight_bytes.expr(meta))
                                * (constant_from!(1) - is_i32.expr(meta))
                                * constant_from!(0xffffffff00000000))
                        + load_value_parsed.expr(meta)
                        - res.expr(meta),
                ]
            }),
        );

        let sp = common_config.sp_cell;
        let eid = common_config.eid_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "load read offset",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| constant_from!(1),
            move |meta| load_base.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_heap_read = allocator.alloc_memory_table_lookup_read_cell(
            "load read data",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta),
            move |meta| constant_from!(0),
            move |meta| load_value_in_heap.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "load write res",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| res.expr(meta),
            move |meta| constant_from!(1),
        );

        let current_memory_page_size = common_config.mpages_cell;

        let address_within_allocated_pages_helper = allocator.alloc_common_range_cell();
        constraint_builder.push(
            "op_load allocated address",
            Box::new(move |meta| {
                let len = constant_from!(1)
                    + is_two_bytes.expr(meta) * constant_from!(1)
                    + is_four_bytes.expr(meta) * constant_from!(3)
                    + is_eight_bytes.expr(meta) * constant_from!(7);

                vec![
                    (load_base.expr(meta)
                        + opcode_load_offset.expr(meta)
                        + len
                        + address_within_allocated_pages_helper.expr(meta)
                        - current_memory_page_size.expr(meta) * constant_from!(WASM_PAGE_SIZE)),
                ]
            }),
        );

        Box::new(LoadConfig {
            opcode_load_offset,
            value_leading_u16_u8_high,
            value_leading_u16_u8_low,
            value_leading_u16_u8_low_diff,
            load_block_index,
            load_inner_pos,
            load_value_in_heap,
            load_value_parsed,
            res,
            load_base,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            is_sign,
            is_i32,
            memory_table_lookup_stack_read,
            memory_table_lookup_heap_read,
            memory_table_lookup_stack_write,
            address_within_allocated_pages_helper,
            value_leading_u16,
            value_flag,
            value_parsed_leading_u8_rem,
            value_parsed_leading_u8_rem_diff,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for LoadConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let load_size = self.is_eight_bytes.expr(meta) * constant_from!(6)
            + self.is_four_bytes.expr(meta) * constant_from!(4)
            + self.is_two_bytes.expr(meta) * constant_from!(2)
            + self.is_sign.expr(meta)
            + constant_from!(1);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Load as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_i32.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + load_size * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.opcode_load_offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match entry.eentry.step_info {
            StepInfo::Load {
                vtype,
                load_size,
                offset,
                raw_address,
                effective_address,
                value,
                block_value1,
                ..
            } => {
                self.opcode_load_offset.assign_u32(ctx, offset)?;

                let len = load_size.byte_size();
                let byte_index = effective_address as u64;
                let inner_byte_index = byte_index & 7;
                let inner_u16_index = inner_byte_index >> 1;

                let value_leading_u16 = if len == 1 {
                    (block_value1 >> (inner_u16_index * 16)) & 0xffff
                } else {
                    (block_value1 >> (inner_u16_index * 16 + (len - 2) * 8)) & 0xffff
                };
                self.value_leading_u16
                    .assign(ctx, value_leading_u16.into())?;
                self.value_leading_u16_u8_high
                    .assign(ctx, (value_leading_u16 >> 8).into())?;
                self.value_leading_u16_u8_low
                    .assign(ctx, (value_leading_u16 & 0xff).into())?;
                self.value_leading_u16_u8_low_diff
                    .assign(ctx, (0xff - (value_leading_u16 & 0xff)).into())?;

                let value_parsed_leading_8 =
                    (block_value1 >> (inner_byte_index * 8 + (len - 1) * 8)) & 0xff;
                self.value_flag
                    .assign(ctx, (value_parsed_leading_8 >> 7).into())?;
                self.value_parsed_leading_u8_rem
                    .assign(ctx, (value_parsed_leading_8 & 0x7f).into())?;
                self.value_parsed_leading_u8_rem_diff
                    .assign(ctx, (0x7f - (value_parsed_leading_8 & 0x7f)).into())?;

                self.load_block_index
                    .assign(ctx, (effective_address as u64) >> 3)?;
                self.load_inner_pos[0].assign_bool(ctx, effective_address & 1 != 0)?;
                self.load_inner_pos[1].assign_bool(ctx, effective_address & 2 != 0)?;
                self.load_inner_pos[2].assign_bool(ctx, effective_address & 4 != 0)?;

                self.load_value_in_heap.assign(ctx, block_value1)?;

                assert!(inner_byte_index == 0 || len != 8);
                assert!(inner_byte_index & 3 == 0 || len != 4);
                assert!(inner_byte_index & 1 == 0 || len != 2);

                let value_parsed = block_value1 >> (inner_byte_index * 8);

                let value_parsed = if len == 8 {
                    value_parsed
                } else {
                    value_parsed & ((1 << (len * 8)) - 1)
                };
                self.load_value_parsed.assign(ctx, value_parsed.into())?;

                self.res.assign(ctx, value.into())?;

                self.is_one_byte.assign_bool(ctx, len == 1)?;
                self.is_two_bytes.assign_bool(ctx, len == 2)?;
                self.is_four_bytes.assign_bool(ctx, len == 4)?;
                self.is_eight_bytes.assign_bool(ctx, len == 8)?;
                self.is_sign.assign_bool(ctx, load_size.is_sign())?;
                self.is_i32.assign_bool(ctx, vtype == VarType::I32)?;

                self.address_within_allocated_pages_helper.assign(
                    ctx,
                    F::from(
                        step.current.allocated_memory_pages as u64 * WASM_PAGE_SIZE
                            - (effective_address as u64 + len),
                    ),
                )?;

                self.load_base.assign(ctx, raw_address as u64)?;

                self.memory_table_lookup_stack_read.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    raw_address as u64,
                )?;

                self.memory_table_lookup_heap_read.assign(
                    ctx,
                    entry.memory_rw_entires[1].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[1].end_eid,
                    effective_address >> 3,
                    LocationType::Heap,
                    false,
                    block_value1,
                )?;

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[2].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    vtype == VarType::I32,
                    value,
                )?;

                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        1
    }
}
