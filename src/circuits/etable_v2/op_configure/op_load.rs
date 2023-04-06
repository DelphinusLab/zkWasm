use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        rtable::pow_table_encode,
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
use num_bigint::BigUint;
use specs::{
    configure_table::WASM_PAGE_SIZE,
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct LoadConfig<F: FieldExt> {
    // offset in opcode
    opcode_load_offset: AllocatedCommonRangeCell<F>,

    // which heap offset to load
    load_block_index: AllocatedCommonRangeCell<F>,
    load_inner_pos: AllocatedU8Cell<F>,
    load_inner_pos_diff: AllocatedU8Cell<F>,

    is_cross_block: AllocatedBitCell<F>,
    cross_block_rem: AllocatedCommonRangeCell<F>,
    cross_block_rem_diff: AllocatedCommonRangeCell<F>,

    load_value_in_heap1: AllocatedU64Cell<F>,
    load_value_in_heap2: AllocatedU64Cell<F>,

    load_tailing: AllocatedU64Cell<F>,
    load_tailing_diff: AllocatedU64Cell<F>,
    load_picked: AllocatedU64Cell<F>,
    load_leading: AllocatedU64Cell<F>,

    load_picked_leading_u16: AllocatedUnlimitedCell<F>,
    load_picked_leading_u16_u8_high: AllocatedU8Cell<F>,
    load_picked_leading_u16_u8_low: AllocatedU8Cell<F>,

    load_picked_flag: AllocatedBitCell<F>,
    load_picked_leading_u8_rem: AllocatedCommonRangeCell<F>,
    load_picked_leading_u8_rem_diff: AllocatedCommonRangeCell<F>,

    res: AllocatedUnlimitedCell<F>,

    // load offset arg
    load_base: AllocatedU64Cell<F>,

    is_one_byte: AllocatedBitCell<F>,
    is_two_bytes: AllocatedBitCell<F>,
    is_four_bytes: AllocatedBitCell<F>,
    is_eight_bytes: AllocatedBitCell<F>,
    len: AllocatedUnlimitedCell<F>,
    len_modulus: AllocatedUnlimitedCell<F>,

    is_sign: AllocatedBitCell<F>,
    is_i32: AllocatedBitCell<F>,

    pos_modulus: AllocatedU64Cell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read1: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read2: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,

    lookup_pow: AllocatedUnlimitedCell<F>,

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

        // which heap offset to load
        let load_block_index = allocator.alloc_common_range_cell();
        let load_inner_pos = allocator.alloc_u8_cell();
        let load_inner_pos_diff = allocator.alloc_u8_cell();
        let is_cross_block = allocator.alloc_bit_cell();
        let cross_block_rem = allocator.alloc_common_range_cell();
        let cross_block_rem_diff = allocator.alloc_common_range_cell();

        let is_one_byte = allocator.alloc_bit_cell();
        let is_two_bytes = allocator.alloc_bit_cell();
        let is_four_bytes = allocator.alloc_bit_cell();
        let is_eight_bytes = allocator.alloc_bit_cell();
        let len = allocator.alloc_unlimited_cell();
        let len_modulus = allocator.alloc_unlimited_cell();

        let load_value_in_heap1 = allocator.alloc_u64_cell();
        let load_value_in_heap2 = allocator.alloc_u64_cell();

        let load_tailing = allocator.alloc_u64_cell();
        let load_tailing_diff = allocator.alloc_u64_cell();
        let load_picked = allocator.alloc_u64_cell();
        let load_leading = allocator.alloc_u64_cell();

        let pos_modulus = allocator.alloc_u64_cell();

        let load_picked_leading_u16 = allocator.alloc_unlimited_cell();
        let load_picked_leading_u16_u8_high = allocator.alloc_u8_cell();
        let load_picked_leading_u16_u8_low = allocator.alloc_u8_cell();

        let load_picked_flag = allocator.alloc_bit_cell();
        let load_picked_leading_u8_rem = allocator.alloc_common_range_cell();
        let load_picked_leading_u8_rem_diff = allocator.alloc_common_range_cell();

        // value after extension
        let res = allocator.alloc_unlimited_cell();

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
            "op_load len",
            Box::new(move |meta| {
                vec![
                    len.expr(meta)
                        - constant_from!(1)
                        - is_two_bytes.expr(meta)
                        - constant_from!(3) * is_four_bytes.expr(meta)
                        - constant_from!(7) * is_eight_bytes.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load load_block_index",
            Box::new(move |meta| {
                vec![
                    load_block_index.expr(meta) * constant_from!(8) + load_inner_pos.expr(meta)
                        - opcode_load_offset.expr(meta)
                        - load_base.expr(meta),
                    load_inner_pos.expr(meta) + load_inner_pos_diff.expr(meta) - constant_from!(7),
                ]
            }),
        );

        constraint_builder.push(
            "op_load cross_block",
            Box::new(move |meta| {
                vec![
                    is_cross_block.expr(meta) * constant_from!(8) + cross_block_rem.expr(meta)
                        - load_inner_pos.expr(meta)
                        - len.expr(meta)
                        + constant_from!(1),
                    cross_block_rem.expr(meta) + cross_block_rem_diff.expr(meta)
                        - constant_from!(7),
                    (is_cross_block.expr(meta) - constant_from!(1))
                        * load_value_in_heap2.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick value",
            Box::new(move |meta| {
                vec![
                    len_modulus.expr(meta)
                        - is_one_byte.expr(meta) * constant_from!(1u64 << 8)
                        - is_two_bytes.expr(meta) * constant_from!(1u64 << 16)
                        - is_four_bytes.expr(meta) * constant_from!(1u64 << 32)
                        - is_eight_bytes.expr(meta)
                            * constant_from_bn!(&(BigUint::from(1u64) << 64)),
                    load_tailing.expr(meta)
                        + load_picked.expr(meta) * pos_modulus.expr(meta)
                        + load_leading.expr(meta) * pos_modulus.expr(meta) * len_modulus.expr(meta)
                        - load_value_in_heap1.expr(meta)
                        - load_value_in_heap2.expr(meta)
                            * constant_from_bn!(&(BigUint::from(1u64) << 64)),
                    load_tailing.expr(meta) + load_tailing_diff.expr(meta) + constant_from!(1)
                        - pos_modulus.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick value size check",
            Box::new(move |meta| {
                vec![
                    is_four_bytes.expr(meta)
                        * (load_picked.u16_cells_le[2].expr(meta)
                            + load_picked.u16_cells_le[3].expr(meta)),
                    is_two_bytes.expr(meta)
                        * (load_picked.expr(meta) - load_picked_leading_u16.expr(meta)),
                    is_one_byte.expr(meta) * (load_picked_leading_u16_u8_high.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick u16 decompose1",
            Box::new(move |meta| {
                vec![
                    load_picked_leading_u16.expr(meta)
                        - (is_two_bytes.expr(meta) + is_one_byte.expr(meta))
                            * load_picked.u16_cells_le[0].expr(meta)
                        - is_four_bytes.expr(meta) * load_picked.u16_cells_le[1].expr(meta)
                        - is_eight_bytes.expr(meta) * load_picked.u16_cells_le[3].expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load pick u16 decompose2",
            Box::new(move |meta| {
                vec![
                    load_picked_leading_u16_u8_high.expr(meta) * constant_from!(1 << 8)
                        + load_picked_leading_u16_u8_low.expr(meta)
                        - load_picked_leading_u16.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_load flag",
            Box::new(move |meta| {
                let value_leading_u8 = is_one_byte.expr(meta)
                    * load_picked_leading_u16_u8_low.expr(meta)
                    + (constant_from!(1) - is_one_byte.expr(meta))
                        * load_picked_leading_u16_u8_high.expr(meta);

                vec![
                    load_picked_flag.expr(meta) * constant_from!(128)
                        + load_picked_leading_u8_rem.expr(meta)
                        - value_leading_u8,
                    load_picked_leading_u8_rem.expr(meta)
                        + load_picked_leading_u8_rem_diff.expr(meta)
                        - constant_from!(127),
                ]
            }),
        );

        constraint_builder.push(
            "op_load extension",
            Box::new(move |meta| {
                vec![
                    is_sign.expr(meta)
                        * load_picked_flag.expr(meta)
                        * (is_one_byte.expr(meta) * constant_from!(0xffffff00)
                            + is_two_bytes.expr(meta) * constant_from!(0xffff0000)
                            + (constant_from!(1) - is_eight_bytes.expr(meta))
                                * (constant_from!(1) - is_i32.expr(meta))
                                * constant_from!(0xffffffff00000000))
                        + load_picked.expr(meta)
                        - res.expr(meta),
                ]
            }),
        );

        let lookup_pow = common_config.pow_table_lookup_cell;

        constraint_builder.push(
            "op_load pos_modulus",
            Box::new(move |meta| {
                vec![
                    lookup_pow.expr(meta)
                        - pow_table_encode(
                            pos_modulus.expr(meta),
                            load_inner_pos.expr(meta) * constant_from!(8),
                        ),
                ]
            }),
        );

        let sp = common_config.sp_cell;
        let eid = common_config.eid_cell;

        let memory_table_lookup_stack_read = allocator.alloc_memory_table_lookup_read_cell(
            "load read offset",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |____| constant_from!(1),
            move |meta| load_base.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_heap_read1 = allocator.alloc_memory_table_lookup_read_cell(
            "load read data1",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta),
            move |____| constant_from!(0),
            move |meta| load_value_in_heap1.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_heap_read2 = allocator.alloc_memory_table_lookup_read_cell(
            "load read data2",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta) + constant_from!(1),
            move |____| constant_from!(0),
            move |meta| load_value_in_heap2.expr(meta),
            move |meta| is_cross_block.expr(meta),
        );

        let memory_table_lookup_stack_write = allocator.alloc_memory_table_lookup_write_cell(
            "load write res",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| res.expr(meta),
            move |____| constant_from!(1),
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
            load_block_index,
            load_inner_pos,
            load_inner_pos_diff,
            is_cross_block,
            cross_block_rem,
            cross_block_rem_diff,
            load_value_in_heap2,
            load_tailing,
            load_picked,
            load_leading,
            load_picked_leading_u16,
            load_picked_leading_u16_u8_high,
            load_picked_leading_u16_u8_low,
            load_picked_flag,
            load_picked_leading_u8_rem,
            load_picked_leading_u8_rem_diff,
            res,
            load_base,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            len,
            len_modulus,
            is_sign,
            is_i32,
            memory_table_lookup_stack_read,
            memory_table_lookup_heap_read1,
            memory_table_lookup_heap_read2,
            memory_table_lookup_stack_write,
            lookup_pow,
            address_within_allocated_pages_helper,
            load_value_in_heap1,
            pos_modulus,
            load_tailing_diff,
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
                block_value2,
            } => {
                self.opcode_load_offset.assign_u32(ctx, offset)?;

                let len = load_size.byte_size();
                let byte_index = effective_address as u64;
                let inner_byte_index = byte_index & 7;

                self.load_block_index
                    .assign_u32(ctx, (effective_address as u32) >> 3)?;
                self.load_inner_pos
                    .assign_u32(ctx, inner_byte_index as u32)?;
                self.load_inner_pos_diff
                    .assign_u32(ctx, 7 - inner_byte_index as u32)?;

                let is_cross_block = (effective_address as u64 & 7) + len > 8;

                let len_modulus = BigUint::from(1u64) << (len * 8);
                let pos_modulus = 1 << (inner_byte_index * 8);
                self.pos_modulus.assign(ctx, pos_modulus.into())?;
                self.lookup_pow.assign_bn(
                    ctx,
                    &((BigUint::from(1u64) << (inner_byte_index * 8 + 16)) + inner_byte_index * 8),
                )?;

                self.is_cross_block.assign_bool(ctx, is_cross_block)?;
                let rem = ((effective_address as u64 & 7) + len - 1) & 7;
                self.cross_block_rem.assign(ctx, rem.into())?;
                self.cross_block_rem_diff.assign(ctx, (7 - rem).into())?;

                self.load_value_in_heap1.assign(ctx, block_value1)?;
                self.load_value_in_heap2.assign(ctx, block_value2)?;

                let tailing_bits = inner_byte_index * 8;
                let picked_bits = len * 8;
                let load_value: BigUint = (BigUint::from(block_value2) << 64) + block_value1;
                let tailing: u64 = load_value.to_u64_digits().first().unwrap_or(&0u64).clone()
                    & ((1 << tailing_bits) - 1);
                let picked: u64 = ((&load_value >> tailing_bits)
                    & ((BigUint::from(1u64) << picked_bits) - 1u64))
                    .to_u64_digits()
                    .first()
                    .unwrap_or(&0u64)
                    .clone();
                let leading: u64 = (load_value >> (picked_bits + tailing_bits))
                    .to_u64_digits()
                    .first()
                    .unwrap_or(&0u64)
                    .clone();

                self.load_tailing.assign(ctx, tailing)?;
                self.load_tailing_diff
                    .assign(ctx, pos_modulus - 1 - tailing)?;
                self.load_picked.assign(ctx, picked)?;
                self.load_leading.assign(ctx, leading)?;

                let load_picked_leading_u16 = if len == 1 {
                    picked
                } else {
                    picked >> (8 * (len - 2))
                };
                self.load_picked_leading_u16
                    .assign(ctx, load_picked_leading_u16.into())?;
                self.load_picked_leading_u16_u8_high
                    .assign(ctx, (load_picked_leading_u16 >> 8).into())?;
                self.load_picked_leading_u16_u8_low
                    .assign(ctx, (load_picked_leading_u16 & 0xff).into())?;

                let load_picked_leading_u8 = if len == 1 {
                    load_picked_leading_u16 & 0xff
                } else {
                    load_picked_leading_u16 >> 8
                };
                self.load_picked_flag
                    .assign(ctx, (load_picked_leading_u8 >> 7).into())?;
                self.load_picked_leading_u8_rem
                    .assign(ctx, (load_picked_leading_u8 & 0x7f).into())?;
                self.load_picked_leading_u8_rem_diff
                    .assign(ctx, (0x7f - (load_picked_leading_u8 & 0x7f)).into())?;

                self.res.assign(ctx, value.into())?;
                self.load_base.assign(ctx, raw_address as u64)?;

                self.is_one_byte.assign_bool(ctx, len == 1)?;
                self.is_two_bytes.assign_bool(ctx, len == 2)?;
                self.is_four_bytes.assign_bool(ctx, len == 4)?;
                self.is_eight_bytes.assign_bool(ctx, len == 8)?;
                self.len.assign(ctx, len.into())?;
                self.len_modulus.assign_bn(ctx, &len_modulus)?;

                self.is_sign.assign_bool(ctx, load_size.is_sign())?;
                self.is_i32.assign_bool(ctx, vtype == VarType::I32)?;

                self.address_within_allocated_pages_helper.assign(
                    ctx,
                    F::from(
                        step.current.allocated_memory_pages as u64 * WASM_PAGE_SIZE
                            - (effective_address as u64 + len),
                    ),
                )?;

                let mut i = 0;
                self.memory_table_lookup_stack_read.assign(
                    ctx,
                    entry.memory_rw_entires[i].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[i].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    true,
                    raw_address as u64,
                )?;
                i += 1;

                self.memory_table_lookup_heap_read1.assign(
                    ctx,
                    entry.memory_rw_entires[i].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[i].end_eid,
                    effective_address >> 3,
                    LocationType::Heap,
                    false,
                    block_value1,
                )?;
                i += 1;

                if is_cross_block {
                    self.memory_table_lookup_heap_read2.assign(
                        ctx,
                        entry.memory_rw_entires[i].start_eid,
                        step.current.eid,
                        entry.memory_rw_entires[i].end_eid,
                        (effective_address >> 3) + 1,
                        LocationType::Heap,
                        false,
                        block_value2,
                    )?;
                    i += 1;
                }

                self.memory_table_lookup_stack_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[i].end_eid,
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

    fn memory_writing_ops(&self, _: &EventTableEntry) -> u32 {
        1
    }
}
