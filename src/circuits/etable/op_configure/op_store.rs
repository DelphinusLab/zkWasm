use crate::{
    circuits::{
        cell::*,
        etable::{
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

pub struct StoreConfig<F: FieldExt> {
    // offset in opcode
    opcode_store_offset: AllocatedCommonRangeCell<F>,

    // which heap offset to load
    load_block_index: AllocatedCommonRangeCell<F>,
    load_block_inner_pos_bits: [AllocatedBitCell<F>; 3],
    load_block_inner_pos: AllocatedUnlimitedCell<F>,

    is_cross_block: AllocatedBitCell<F>,
    cross_block_rem: AllocatedCommonRangeCell<F>,
    cross_block_rem_diff: AllocatedCommonRangeCell<F>,

    load_value_in_heap1: AllocatedU64Cell<F>,
    load_value_in_heap2: AllocatedU64Cell<F>,

    load_tailing: AllocatedU64Cell<F>,
    load_tailing_diff: AllocatedU64Cell<F>,
    load_picked: AllocatedU64Cell<F>,
    load_leading: AllocatedU64Cell<F>,
    load_picked_byte_proof: AllocatedU8Cell<F>,

    pos_modulus: AllocatedU64Cell<F>,

    unchanged_value: AllocatedUnlimitedCell<F>,
    len: AllocatedUnlimitedCell<F>,
    len_modulus: AllocatedUnlimitedCell<F>,

    store_value: AllocatedU64Cell<F>,
    store_value_tailing_u16_u8_high: AllocatedU8Cell<F>,
    store_value_tailing_u16_u8_low: AllocatedU8Cell<F>,
    store_value_wrapped: AllocatedUnlimitedCell<F>,

    store_value_in_heap1: AllocatedU64Cell<F>,
    store_value_in_heap2: AllocatedU64Cell<F>,

    // load offset arg
    store_base: AllocatedCommonRangeCell<F>,

    is_one_byte: AllocatedBitCell<F>,
    is_two_bytes: AllocatedBitCell<F>,
    is_four_bytes: AllocatedBitCell<F>,
    is_eight_bytes: AllocatedBitCell<F>,
    is_i32: AllocatedBitCell<F>,

    memory_table_lookup_stack_read_pos: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_val: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read1: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read2: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_write1: AllocatedMemoryTableLookupWriteCell<F>,
    memory_table_lookup_heap_write2: AllocatedMemoryTableLookupWriteCell<F>,

    lookup_pow: AllocatedUnlimitedCell<F>,

    address_within_allocated_pages_helper: AllocatedCommonRangeCell<F>,
}

pub struct StoreConfigBuilder;

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for StoreConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let opcode_store_offset = allocator.alloc_common_range_cell();
        let store_base = allocator.alloc_common_range_cell();

        // which heap offset to load
        let load_block_index = allocator.alloc_common_range_cell();
        let load_block_inner_pos_bits = [0; 3].map(|_| allocator.alloc_bit_cell());
        let load_block_inner_pos = allocator.alloc_unlimited_cell();
        let is_cross_block = allocator.alloc_bit_cell();
        let cross_block_rem = allocator.alloc_common_range_cell();
        let cross_block_rem_diff = allocator.alloc_common_range_cell();

        let len = allocator.alloc_unlimited_cell();
        let len_modulus = allocator.alloc_unlimited_cell();

        let load_value_in_heap1 = allocator.alloc_u64_cell();
        let load_value_in_heap2 = allocator.alloc_u64_cell();

        let load_tailing = allocator.alloc_u64_cell();
        let load_tailing_diff = allocator.alloc_u64_cell();
        let load_picked = allocator.alloc_u64_cell();
        let load_picked_byte_proof = allocator.alloc_u8_cell();
        let load_leading = allocator.alloc_u64_cell();

        let pos_modulus = allocator.alloc_u64_cell();

        let store_value = allocator.alloc_u64_cell();
        let store_value_wrapped = allocator.alloc_unlimited_cell();

        let is_one_byte = allocator.alloc_bit_cell();
        let is_two_bytes = allocator.alloc_bit_cell();
        let is_four_bytes = allocator.alloc_bit_cell();
        let is_eight_bytes = allocator.alloc_bit_cell();
        let is_i32 = allocator.alloc_bit_cell();

        constraint_builder.push(
            "op_store length",
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
            "op_store len",
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
            "op_store load_block_index",
            Box::new(move |meta| {
                vec![
                    load_block_index.expr(meta) * constant_from!(8)
                        + load_block_inner_pos.expr(meta)
                        - opcode_store_offset.expr(meta)
                        - store_base.expr(meta),
                    load_block_inner_pos.expr(meta)
                        - load_block_inner_pos_bits[0].expr(meta)
                        - load_block_inner_pos_bits[1].expr(meta) * constant_from!(2)
                        - load_block_inner_pos_bits[2].expr(meta) * constant_from!(4),
                ]
            }),
        );

        constraint_builder.push(
            "op_store cross_block",
            Box::new(move |meta| {
                vec![
                    is_cross_block.expr(meta) * constant_from!(8) + cross_block_rem.expr(meta)
                        - load_block_inner_pos.expr(meta)
                        - len.expr(meta)
                        + constant_from!(1),
                    cross_block_rem.expr(meta) + cross_block_rem_diff.expr(meta)
                        - constant_from!(7),
                    (is_cross_block.expr(meta) - constant_from!(1))
                        * load_value_in_heap2.expr(meta),
                ]
            }),
        );

        let store_value_in_heap1 = allocator.alloc_u64_cell();
        let store_value_in_heap2 = allocator.alloc_u64_cell();

        let unchanged_value = allocator.alloc_unlimited_cell();

        constraint_builder.push(
            "op_store len modulus",
            Box::new(move |meta| {
                vec![
                    len_modulus.expr(meta)
                        - is_one_byte.expr(meta) * constant_from!(1u64 << 8)
                        - is_two_bytes.expr(meta) * constant_from!(1u64 << 16)
                        - is_four_bytes.expr(meta) * constant_from!(1u64 << 32)
                        - is_eight_bytes.expr(meta)
                            * constant_from_bn!(&(BigUint::from(1u64) << 64)),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pick value1",
            Box::new(move |meta| {
                vec![
                    unchanged_value.expr(meta)
                        - load_tailing.expr(meta)
                        - load_leading.expr(meta) * pos_modulus.expr(meta) * len_modulus.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pick value2",
            Box::new(move |meta| {
                vec![
                    unchanged_value.expr(meta) + load_picked.expr(meta) * pos_modulus.expr(meta)
                        - load_value_in_heap1.expr(meta)
                        - load_value_in_heap2.expr(meta)
                            * constant_from_bn!(&(BigUint::from(1u64) << 64)),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pick value3",
            Box::new(move |meta| {
                vec![
                    unchanged_value.expr(meta)
                        + store_value_wrapped.expr(meta) * pos_modulus.expr(meta)
                        - store_value_in_heap1.expr(meta)
                        - store_value_in_heap2.expr(meta)
                            * constant_from_bn!(&(BigUint::from(1u64) << 64)),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pick helper value check",
            Box::new(move |meta| {
                vec![
                    load_tailing.expr(meta) + load_tailing_diff.expr(meta) + constant_from!(1)
                        - pos_modulus.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pick value size check",
            Box::new(move |meta| {
                vec![
                    is_four_bytes.expr(meta)
                        * (load_picked.u16_cells_le[2].expr(meta)
                            + load_picked.u16_cells_le[3].expr(meta)),
                    is_two_bytes.expr(meta)
                        * (load_picked.expr(meta) - load_picked.u16_cells_le[0].expr(meta)),
                    is_one_byte.expr(meta)
                        * (load_picked.expr(meta) - load_picked_byte_proof.expr(meta)),
                ]
            }),
        );

        let store_value_tailing_u16_u8_high = allocator.alloc_u8_cell();
        let store_value_tailing_u16_u8_low = allocator.alloc_u8_cell();

        constraint_builder.push(
            "op_store tailing u16 decompose",
            Box::new(move |meta| {
                vec![
                    store_value_tailing_u16_u8_high.expr(meta) * constant_from!(1 << 8)
                        + store_value_tailing_u16_u8_low.expr(meta)
                        - store_value.u16_cells_le[0].expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store value wrap",
            Box::new(move |meta| {
                vec![
                    store_value_wrapped.expr(meta)
                        - (is_one_byte.expr(meta) * store_value_tailing_u16_u8_low.expr(meta)
                            + is_two_bytes.expr(meta) * store_value.u16_cells_le[0].expr(meta)
                            + is_four_bytes.expr(meta)
                                * (store_value.u16_cells_le[0].expr(meta)
                                    + store_value.u16_cells_le[1].expr(meta)
                                        * constant_from!(1 << 16))
                            + is_eight_bytes.expr(meta) * store_value.expr(meta)),
                ]
            }),
        );

        let lookup_pow = common_config.pow_table_lookup_cell;

        constraint_builder.push(
            "op_store pow lookup",
            Box::new(move |meta| {
                vec![
                    lookup_pow.expr(meta)
                        - pow_table_encode(
                            pos_modulus.expr(meta),
                            load_block_inner_pos.expr(meta) * constant_from!(8),
                        ),
                ]
            }),
        );

        let sp = common_config.sp_cell;
        let eid = common_config.eid_cell;

        let memory_table_lookup_stack_read_val = allocator.alloc_memory_table_lookup_read_cell(
            "store read data",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| store_value.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_stack_read_pos = allocator.alloc_memory_table_lookup_read_cell(
            "store read pos",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |____| constant_from!(1),
            move |meta| store_base.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_heap_read1 = allocator.alloc_memory_table_lookup_read_cell(
            "store load origin1",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta),
            move |____| constant_from!(0),
            move |meta| load_value_in_heap1.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_heap_read2 = allocator.alloc_memory_table_lookup_read_cell(
            "store load origin2",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta) + constant_from!(1),
            move |____| constant_from!(0),
            move |meta| load_value_in_heap2.expr(meta),
            move |meta| is_cross_block.expr(meta),
        );

        let memory_table_lookup_heap_write1 = allocator.alloc_memory_table_lookup_write_cell(
            "store write res1",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta),
            move |____| constant_from!(0),
            move |meta| store_value_in_heap1.expr(meta),
            move |____| constant_from!(1),
        );

        let memory_table_lookup_heap_write2 = allocator.alloc_memory_table_lookup_write_cell(
            "store write res1",
            constraint_builder,
            eid,
            move |____| constant_from!(LocationType::Heap as u64),
            move |meta| load_block_index.expr(meta) + constant_from!(1),
            move |____| constant_from!(0),
            move |meta| store_value_in_heap2.expr(meta),
            move |meta| is_cross_block.expr(meta),
        );

        let current_memory_page_size = common_config.mpages_cell;

        let address_within_allocated_pages_helper = allocator.alloc_common_range_cell();
        constraint_builder.push(
            "op_store allocated address",
            Box::new(move |meta| {
                let len = constant_from!(1)
                    + is_two_bytes.expr(meta) * constant_from!(1)
                    + is_four_bytes.expr(meta) * constant_from!(3)
                    + is_eight_bytes.expr(meta) * constant_from!(7);

                vec![
                    (store_base.expr(meta)
                        + opcode_store_offset.expr(meta)
                        + len
                        + address_within_allocated_pages_helper.expr(meta)
                        - current_memory_page_size.expr(meta) * constant_from!(WASM_PAGE_SIZE)),
                ]
            }),
        );

        Box::new(StoreConfig {
            opcode_store_offset,
            load_block_index,
            load_block_inner_pos_bits,
            load_block_inner_pos,
            is_cross_block,
            cross_block_rem,
            cross_block_rem_diff,
            load_value_in_heap1,
            load_value_in_heap2,
            load_tailing,
            load_picked,
            load_picked_byte_proof,
            load_leading,
            unchanged_value,
            store_value,
            store_value_tailing_u16_u8_high,
            store_value_tailing_u16_u8_low,
            store_value_wrapped,
            store_value_in_heap1,
            store_value_in_heap2,
            store_base,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            is_i32,
            memory_table_lookup_stack_read_pos,
            memory_table_lookup_stack_read_val,
            memory_table_lookup_heap_read1,
            memory_table_lookup_heap_read2,
            memory_table_lookup_heap_write1,
            memory_table_lookup_heap_write2,
            lookup_pow,
            address_within_allocated_pages_helper,
            load_tailing_diff,
            pos_modulus,
            len,
            len_modulus,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for StoreConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let store_size = self.is_eight_bytes.expr(meta) * constant_from!(3)
            + self.is_four_bytes.expr(meta) * constant_from!(2)
            + self.is_two_bytes.expr(meta) * constant_from!(1)
            + constant_from!(1);

        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Store as u64) << OPCODE_CLASS_SHIFT)
        )) + self.is_i32.expr(meta)
            * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG0_SHIFT)))
            + store_size * constant!(bn_to_field(&(BigUint::from(1u64) << OPCODE_ARG1_SHIFT)))
            + self.opcode_store_offset.expr(meta)
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match entry.eentry.step_info {
            StepInfo::Store {
                vtype,
                store_size,
                offset,
                raw_address,
                effective_address,
                pre_block_value1,
                updated_block_value1,
                pre_block_value2,
                updated_block_value2,
                value,
            } => {
                self.opcode_store_offset.assign_u32(ctx, offset)?;

                let len = store_size.byte_size();
                self.len.assign(ctx, len.into())?;

                let byte_index = effective_address as u64;
                let inner_byte_index = byte_index & 7;

                self.load_block_index
                    .assign_u32(ctx, (effective_address as u32) >> 3)?;
                self.load_block_inner_pos
                    .assign_u32(ctx, inner_byte_index as u32)?;
                self.load_block_inner_pos_bits[0].assign_bool(ctx, inner_byte_index & 1 != 0)?;
                self.load_block_inner_pos_bits[1].assign_bool(ctx, inner_byte_index & 2 != 0)?;
                self.load_block_inner_pos_bits[2].assign_bool(ctx, inner_byte_index & 4 != 0)?;

                let is_cross_block = (effective_address as u64 & 7) + len > 8;

                let len_modulus = BigUint::from(1u64) << (len * 8);
                self.len_modulus.assign_bn(ctx, &len_modulus)?;

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

                self.load_value_in_heap1.assign(ctx, pre_block_value1)?;
                self.load_value_in_heap2.assign(ctx, pre_block_value2)?;

                let tailing_bits = inner_byte_index * 8;
                let picked_bits = len * 8;
                let load_value: BigUint =
                    (BigUint::from(pre_block_value2) << 64) + pre_block_value1;
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
                if len == 1 {
                    self.load_picked_byte_proof.assign(ctx, picked.into())?;
                }
                self.load_leading.assign(ctx, leading)?;

                self.unchanged_value.assign_bn(
                    ctx,
                    &((BigUint::from(leading) << ((inner_byte_index + len) * 8)) + tailing),
                )?;

                self.store_value.assign(ctx, value)?;
                self.store_value_tailing_u16_u8_low
                    .assign(ctx, (value & 0xff).into())?;
                self.store_value_tailing_u16_u8_high
                    .assign(ctx, ((value >> 8) & 0xff).into())?;
                let value_wrapped = if len == 8 {
                    value
                } else {
                    value & ((1 << (len * 8)) - 1)
                };
                self.store_value_wrapped.assign(ctx, value_wrapped.into())?;
                self.store_value_in_heap1
                    .assign(ctx, updated_block_value1)?;
                self.store_value_in_heap2
                    .assign(ctx, updated_block_value2)?;

                self.is_one_byte.assign_bool(ctx, len == 1)?;
                self.is_two_bytes.assign_bool(ctx, len == 2)?;
                self.is_four_bytes.assign_bool(ctx, len == 4)?;
                self.is_eight_bytes.assign_bool(ctx, len == 8)?;
                self.is_i32.assign_bool(ctx, vtype == VarType::I32)?;

                self.lookup_pow.assign_bn(
                    ctx,
                    &((BigUint::from(1u64) << (inner_byte_index * 8 + 16)) + inner_byte_index * 8),
                )?;

                self.address_within_allocated_pages_helper.assign(
                    ctx,
                    F::from(
                        step.current.allocated_memory_pages as u64 * WASM_PAGE_SIZE
                            - (effective_address as u64 + len),
                    ),
                )?;

                self.store_base.assign_u32(ctx, raw_address)?;

                self.memory_table_lookup_stack_read_val.assign(
                    ctx,
                    entry.memory_rw_entires[0].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[0].end_eid,
                    step.current.sp + 1,
                    LocationType::Stack,
                    vtype == VarType::I32,
                    value as u64,
                )?;

                self.memory_table_lookup_stack_read_pos.assign(
                    ctx,
                    entry.memory_rw_entires[1].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[1].end_eid,
                    step.current.sp + 2,
                    LocationType::Stack,
                    true,
                    raw_address as u64,
                )?;

                self.memory_table_lookup_heap_read1.assign(
                    ctx,
                    entry.memory_rw_entires[2].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[2].end_eid,
                    effective_address >> 3,
                    LocationType::Heap,
                    false,
                    pre_block_value1,
                )?;

                self.memory_table_lookup_heap_write1.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[3].end_eid,
                    effective_address >> 3,
                    LocationType::Heap,
                    false,
                    updated_block_value1,
                )?;

                if is_cross_block {
                    self.memory_table_lookup_heap_read2.assign(
                        ctx,
                        entry.memory_rw_entires[4].start_eid,
                        step.current.eid,
                        entry.memory_rw_entires[4].end_eid,
                        (effective_address >> 3) + 1,
                        LocationType::Heap,
                        false,
                        pre_block_value2,
                    )?;

                    self.memory_table_lookup_heap_write2.assign(
                        ctx,
                        step.current.eid,
                        entry.memory_rw_entires[5].end_eid,
                        (effective_address >> 3) + 1,
                        LocationType::Heap,
                        false,
                        updated_block_value2,
                    )?;
                }
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1) + self.is_cross_block.expr(meta))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        match entry.step_info {
            StepInfo::Store {
                store_size,
                effective_address,
                ..
            } => {
                let is_cross_block = (effective_address as u64 & 7) + store_size.byte_size() > 8;
                if is_cross_block {
                    2
                } else {
                    1
                }
            }
            _ => unreachable!(),
        }
    }
}
