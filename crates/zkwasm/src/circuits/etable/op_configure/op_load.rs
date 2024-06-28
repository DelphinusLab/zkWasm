use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::mtable::utils::block_from_address;
use crate::circuits::mtable::utils::byte_offset_from_address;
use crate::circuits::mtable::utils::WASM_BLOCKS_PER_PAGE;
use crate::circuits::mtable::utils::WASM_BLOCK_BYTE_OFFSET_MASK;
use crate::circuits::mtable::utils::WASM_BLOCK_BYTE_SIZE;
use crate::circuits::rtable::pow_table_power_encode;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant;
use crate::constant_from;
use crate::constant_from_bn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::etable::EventTableEntry;
use specs::itable::OpcodeClass;
use specs::itable::OPCODE_ARG0_SHIFT;
use specs::itable::OPCODE_ARG1_SHIFT;
use specs::itable::OPCODE_CLASS_SHIFT;
use specs::mtable::LocationType;
use specs::mtable::VarType;
use specs::step::StepInfo;

pub struct LoadConfig<F: FieldExt> {
    // offset in opcode
    opcode_load_offset: AllocatedU32Cell<F>,

    // which heap offset to load
    load_block_index: AllocatedU32Cell<F>,
    load_inner_pos: AllocatedU8Cell<F>,
    /// helper to prove load_inner_pos < WASM_BLOCK_BYTE_SIZE
    load_inner_pos_diff: AllocatedU8Cell<F>,

    is_cross_block: AllocatedBitCell<F>,
    cross_block_rem: AllocatedCommonRangeCell<F>,
    /// helper to prove cross_block_rem < WASM_BLOCK_BYTE_SIZE
    cross_block_rem_diff: AllocatedCommonRangeCell<F>,

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

    is_one_byte: AllocatedBitCell<F>,
    is_two_bytes: AllocatedBitCell<F>,
    is_four_bytes: AllocatedBitCell<F>,
    is_eight_bytes: AllocatedBitCell<F>,
    bytes: AllocatedUnlimitedCell<F>,
    len_modulus: AllocatedUnlimitedCell<F>,

    is_sign: AllocatedBitCell<F>,
    is_i32: AllocatedBitCell<F>,

    memory_table_lookup_stack_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read1: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read2: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_write: AllocatedMemoryTableLookupWriteCell<F>,

    lookup_pow_modulus: AllocatedUnlimitedCell<F>,
    lookup_pow_power: AllocatedUnlimitedCell<F>,

    address_within_allocated_pages_helper: AllocatedCommonRangeCell<F>,

    degree_helper: AllocatedBitCell<F>,
}

pub struct LoadConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for LoadConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let opcode_load_offset = allocator.alloc_u32_cell();

        // which heap offset to load
        let load_block_index = allocator.alloc_u32_cell();
        let load_inner_pos = allocator.alloc_u8_cell();
        let load_inner_pos_diff = allocator.alloc_u8_cell();
        let is_cross_block = allocator.alloc_bit_cell();
        let cross_block_rem = allocator.alloc_common_range_cell();
        let cross_block_rem_diff = allocator.alloc_common_range_cell();

        let is_one_byte = allocator.alloc_bit_cell();
        let is_two_bytes = allocator.alloc_bit_cell();
        let is_four_bytes = allocator.alloc_bit_cell();
        let is_eight_bytes = allocator.alloc_bit_cell();
        let bytes = allocator.alloc_unlimited_cell();
        let len_modulus = allocator.alloc_unlimited_cell();

        let load_tailing = allocator.alloc_u64_cell();
        let load_tailing_diff = allocator.alloc_u64_cell();
        let load_picked = allocator.alloc_u64_cell();
        let load_leading = allocator.alloc_u64_cell();

        let lookup_pow_modulus = common_config.pow_table_lookup_modulus_cell;

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

        let degree_helper = allocator.alloc_bit_cell();

        let sp = common_config.sp_cell;
        let eid = common_config.eid_cell;

        let memory_table_lookup_stack_read = allocator
            .alloc_memory_table_lookup_read_cell_with_value(
                "load read offset",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Stack as u64),
                move |meta| sp.expr(meta) + constant_from!(1),
                move |____| constant_from!(1),
                move |____| constant_from!(1),
            );

        let memory_table_lookup_heap_read1 = allocator
            .alloc_memory_table_lookup_read_cell_with_value(
                "load read data1",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Heap as u64),
                move |meta| load_block_index.expr(meta),
                move |____| constant_from!(0),
                move |____| constant_from!(1),
            );

        let memory_table_lookup_heap_read2 = allocator
            .alloc_memory_table_lookup_read_cell_with_value(
                "load read data2",
                constraint_builder,
                eid,
                move |____| constant_from!(LocationType::Heap as u64),
                move |meta| load_block_index.expr(meta) + constant_from!(1),
                move |____| constant_from!(0),
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

        let load_base = memory_table_lookup_stack_read.value_cell;
        let load_value_in_heap1 = memory_table_lookup_heap_read1.value_cell;
        let load_value_in_heap2 = memory_table_lookup_heap_read2.value_cell;

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
            "op_load bytes",
            Box::new(move |meta| {
                vec![
                    bytes.expr(meta)
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
                    load_block_index.expr(meta) * constant_from!(WASM_BLOCK_BYTE_SIZE)
                        + load_inner_pos.expr(meta)
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
                    is_cross_block.expr(meta) * constant_from!(WASM_BLOCK_BYTE_SIZE)
                        + cross_block_rem.expr(meta)
                        - load_inner_pos.expr(meta)
                        - bytes.expr(meta)
                        + constant_from!(1),
                    cross_block_rem.expr(meta) + cross_block_rem_diff.expr(meta)
                        - constant_from!(WASM_BLOCK_BYTE_SIZE - 1),
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
                        + load_picked.expr(meta) * lookup_pow_modulus.expr(meta)
                        + load_leading.expr(meta)
                            * lookup_pow_modulus.expr(meta)
                            * len_modulus.expr(meta)
                        - load_value_in_heap1.expr(meta)
                        - load_value_in_heap2.expr(meta)
                            * constant_from_bn!(&(BigUint::from(1u64) << 64)),
                    load_tailing.expr(meta) + load_tailing_diff.expr(meta) + constant_from!(1)
                        - lookup_pow_modulus.expr(meta),
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
                    is_one_byte.expr(meta)
                        * (load_picked.expr(meta) - load_picked_leading_u16_u8_low.expr(meta)),
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
                    load_picked_flag.expr(meta) * is_sign.expr(meta) - degree_helper.expr(meta),
                    degree_helper.expr(meta)
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

        let lookup_pow_power = common_config.pow_table_lookup_power_cell;

        constraint_builder.push(
            "op_load pos_modulus",
            Box::new(move |meta| {
                vec![
                    lookup_pow_power.expr(meta)
                        - pow_table_power_encode(load_inner_pos.expr(meta) * constant_from!(8)),
                ]
            }),
        );

        let current_memory_page_size = common_config.mpages_cell;

        let address_within_allocated_pages_helper = allocator.alloc_common_range_cell();
        constraint_builder.push(
            "op_load allocated address",
            Box::new(move |meta| {
                vec![
                    (load_block_index.expr(meta)
                        + is_cross_block.expr(meta)
                        + constant_from!(1)
                        + address_within_allocated_pages_helper.expr(meta)
                        - current_memory_page_size.expr(meta)
                            * constant_from!(WASM_BLOCKS_PER_PAGE)),
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
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            bytes,
            len_modulus,
            is_sign,
            is_i32,
            memory_table_lookup_stack_read,
            memory_table_lookup_heap_read1,
            memory_table_lookup_heap_read2,
            memory_table_lookup_stack_write,
            lookup_pow_power,
            address_within_allocated_pages_helper,
            lookup_pow_modulus,
            load_tailing_diff,

            degree_helper,
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
        step: &mut StepStatus<F>,
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
                let len = load_size.byte_size();

                self.opcode_load_offset.assign(ctx, offset)?;

                let inner_byte_index = byte_offset_from_address(effective_address);
                let block_start_index = block_from_address(effective_address);

                self.load_block_index.assign(ctx, block_start_index)?;
                self.load_inner_pos.assign_u32(ctx, inner_byte_index)?;
                self.load_inner_pos_diff
                    .assign_u32(ctx, WASM_BLOCK_BYTE_SIZE - 1 - inner_byte_index)?;

                let pos_modulus = 1 << (inner_byte_index * 8);
                self.lookup_pow_modulus.assign(ctx, pos_modulus.into())?;
                self.lookup_pow_power.assign_bn(
                    ctx,
                    &pow_table_power_encode(BigUint::from(inner_byte_index * 8)),
                )?;

                let is_cross_block = inner_byte_index + len > WASM_BLOCK_BYTE_SIZE;
                self.is_cross_block.assign_bool(ctx, is_cross_block)?;
                let rem = (inner_byte_index + len - 1) & WASM_BLOCK_BYTE_OFFSET_MASK;
                self.cross_block_rem.assign_u32(ctx, rem)?;
                self.cross_block_rem_diff
                    .assign_u32(ctx, WASM_BLOCK_BYTE_SIZE - 1 - rem)?;

                let tailing_bits = inner_byte_index * 8;
                let picked_bits = len * 8;
                let load_value: BigUint = (BigUint::from(block_value2) << 64) + block_value1;
                let tailing: u64 = *load_value.to_u64_digits().first().unwrap_or(&0u64)
                    & ((1 << tailing_bits) - 1);
                let picked: u64 = *((&load_value >> tailing_bits)
                    & ((BigUint::from(1u64) << picked_bits) - 1u64))
                    .to_u64_digits()
                    .first()
                    .unwrap_or(&0u64);
                let leading: u64 = *(load_value >> (picked_bits + tailing_bits))
                    .to_u64_digits()
                    .first()
                    .unwrap_or(&0u64);

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

                self.is_one_byte.assign_bool(ctx, len == 1)?;
                self.is_two_bytes.assign_bool(ctx, len == 2)?;
                self.is_four_bytes.assign_bool(ctx, len == 4)?;
                self.is_eight_bytes.assign_bool(ctx, len == 8)?;
                self.bytes.assign(ctx, (len as u64).into())?;
                self.len_modulus
                    .assign_bn(ctx, &(BigUint::from(1u64) << (len * 8)))?;

                self.is_sign.assign_bool(ctx, load_size.is_sign())?;
                self.is_i32.assign_bool(ctx, vtype == VarType::I32)?;

                self.degree_helper.assign(
                    ctx,
                    F::from(load_size.is_sign()) * F::from(load_picked_leading_u8 >> 7),
                )?;

                self.address_within_allocated_pages_helper.assign_u32(
                    ctx,
                    step.current.allocated_memory_pages * WASM_BLOCKS_PER_PAGE
                        - (block_start_index + is_cross_block as u32 + 1),
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
