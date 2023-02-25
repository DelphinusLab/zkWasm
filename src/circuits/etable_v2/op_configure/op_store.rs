use std::marker::PhantomData;

use crate::{
    circuits::{
        cell::*,
        etable_v2::{
            allocator::*, ConstraintBuilder, EventTableCommonConfig, EventTableOpcodeConfig,
            EventTableOpcodeConfigBuilder,
        },
        jtable::{expression::JtableLookupEntryEncode, JumpTableConfig},
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
use num_bigint::{BigUint, ToBigUint};
use specs::{
    configure_table::WASM_PAGE_SIZE,
    encode::{frame_table::encode_frame_table_entry, memory_table, FromBn},
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_ARG0_SHIFT, OPCODE_ARG1_SHIFT, OPCODE_CLASS_SHIFT},
    mtable::{LocationType, VarType},
    step::StepInfo,
};

pub struct StoreConfig<F: FieldExt> {
    // offset in opcode
    opcode_store_offset: AllocatedCommonRangeCell<F>,

    value_leading_u16: AllocatedUnlimitedCell<F>,
    value_leading_u16_u8_high: AllocatedU8Cell<F>,
    value_leading_u16_u8_low: AllocatedU8Cell<F>,

    // which heap offset to load
    store_block_index: AllocatedU64Cell<F>,
    store_inner_pos: [AllocatedBitCell<F>; 3],

    store_value_tailing_u16_u8_high: AllocatedU8Cell<F>,
    store_value_tailing_u16_u8_low: AllocatedU8Cell<F>,

    load_value_in_heap: AllocatedU64Cell<F>,
    // value after pick bytes from load
    load_value_picked: AllocatedU64Cell<F>,

    store_value: AllocatedU64Cell<F>,
    store_value_wrapped: AllocatedU64Cell<F>,
    store_value_modulus: AllocatedU64Cell<F>,

    // value after extension
    res: AllocatedUnlimitedCell<F>,

    // load offset arg
    store_base: AllocatedU64Cell<F>,

    is_one_byte: AllocatedBitCell<F>,
    is_two_bytes: AllocatedBitCell<F>,
    is_four_bytes: AllocatedBitCell<F>,
    is_eight_bytes: AllocatedBitCell<F>,
    is_i32: AllocatedBitCell<F>,

    memory_table_lookup_stack_read_pos: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_stack_read_val: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_read: AllocatedMemoryTableLookupReadCell<F>,
    memory_table_lookup_heap_write: AllocatedMemoryTableLookupWriteCell<F>,

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
        let store_base = allocator.alloc_u64_cell();

        let value_leading_u16 = allocator.alloc_unlimited_cell();
        let value_leading_u16_u8_high = allocator.alloc_u8_cell();
        let value_leading_u16_u8_low = allocator.alloc_u8_cell();

        let store_value_tailing_u16_u8_high = allocator.alloc_u8_cell();
        let store_value_tailing_u16_u8_low = allocator.alloc_u8_cell();

        // which heap offset to load
        let store_block_index = allocator.alloc_u64_cell();
        let store_inner_pos = [0; 3].map(|x| allocator.alloc_bit_cell());

        let load_value_in_heap = allocator.alloc_u64_cell();
        // value after pick bytes from load
        let load_value_picked = allocator.alloc_u64_cell();

        let store_value = allocator.alloc_u64_cell();
        let store_value_wrapped = allocator.alloc_u64_cell();
        let store_value_modulus = allocator.alloc_u64_cell();

        // value after merge
        let res = allocator.alloc_unlimited_cell();

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
            "op_store store target",
            Box::new(move |meta| {
                vec![
                    (store_block_index.expr(meta) * constant_from!(8)
                        + store_inner_pos[2].expr(meta) * constant_from!(4)
                        + store_inner_pos[1].expr(meta) * constant_from!(2)
                        + store_inner_pos[0].expr(meta))
                        - opcode_store_offset.expr(meta)
                        - store_base.expr(meta),
                ]
            }),
        );

        // We only support store aligned in this version
        constraint_builder.push(
            "op_store store align",
            Box::new(move |meta| {
                vec![
                    is_eight_bytes.expr(meta)
                        * (store_inner_pos[2].expr(meta)
                            + store_inner_pos[1].expr(meta)
                            + store_inner_pos[0].expr(meta)),
                    is_four_bytes.expr(meta)
                        * (store_inner_pos[1].expr(meta) + store_inner_pos[0].expr(meta)),
                    is_two_bytes.expr(meta) * (store_inner_pos[0].expr(meta)),
                ]
            }),
        );

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

        constraint_builder.push(
            "op_store pick load leading u16",
            Box::new(move |meta| {
                vec![
                    // ask for a better encoding
                    value_leading_u16.expr(meta)
                        - (is_two_bytes.expr(meta) + is_one_byte.expr(meta))
                            * ((store_inner_pos[2].expr(meta)
                                * store_inner_pos[1].expr(meta)
                                * load_value_in_heap.u16_cells_le[3].expr(meta))
                                + (constant_from!(1) - store_inner_pos[2].expr(meta))
                                    * store_inner_pos[1].expr(meta)
                                    * (load_value_in_heap.u16_cells_le[1].expr(meta))
                                + store_inner_pos[2].expr(meta)
                                    * (constant_from!(1) - store_inner_pos[1].expr(meta))
                                    * (load_value_in_heap.u16_cells_le[2].expr(meta))
                                + (constant_from!(1) - store_inner_pos[2].expr(meta))
                                    * (constant_from!(1) - store_inner_pos[1].expr(meta))
                                    * (load_value_in_heap.u16_cells_le[0].expr(meta)))
                        - is_four_bytes.expr(meta)
                            * (store_inner_pos[2].expr(meta)
                                * load_value_in_heap.u16_cells_le[3].expr(meta)
                                + (constant_from!(1) - store_inner_pos[2].expr(meta))
                                    * load_value_in_heap.u16_cells_le[1].expr(meta))
                        - is_eight_bytes.expr(meta) * load_value_in_heap.u16_cells_le[3].expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store load leading u16 decompose",
            Box::new(move |meta| {
                vec![
                    value_leading_u16_u8_high.expr(meta) * constant_from!(1 << 8)
                        + value_leading_u16_u8_low.expr(meta)
                        - value_leading_u16.expr(meta),
                ]
            }),
        );

        constraint_builder.push(
            "op_store pick load value",
            Box::new(move |meta| {
                vec![
                    load_value_picked.expr(meta)
                        - is_eight_bytes.expr(meta) * load_value_in_heap.expr(meta)
                        - is_four_bytes.expr(meta)
                            * store_inner_pos[2].expr(meta)
                            * (load_value_in_heap.u16_cells_le[2].expr(meta)
                                + load_value_in_heap.u16_cells_le[3].expr(meta)
                                    * constant_from!(1 << 16))
                        - is_four_bytes.expr(meta)
                            * (constant_from!(1) - store_inner_pos[2].expr(meta))
                            * (load_value_in_heap.u16_cells_le[0].expr(meta)
                                + load_value_in_heap.u16_cells_le[1].expr(meta)
                                    * constant_from!(1 << 16))
                        - is_two_bytes.expr(meta) * (value_leading_u16.expr(meta))
                        - is_one_byte.expr(meta)
                            * store_inner_pos[0].expr(meta)
                            * (value_leading_u16_u8_high.expr(meta))
                        - is_one_byte.expr(meta)
                            * (constant_from!(1) - store_inner_pos[0].expr(meta))
                            * (value_leading_u16_u8_low.expr(meta)),
                ]
            }),
        );

        constraint_builder.push(
            "op_store merge value",
            Box::new(move |meta| {
                vec![
                    res.expr(meta)
                        - (load_value_in_heap.expr(meta)
                            + (store_value_wrapped.expr(meta) - load_value_picked.expr(meta))
                                * store_value_modulus.expr(meta)),
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
                            store_value_modulus.expr(meta),
                            store_inner_pos[0].expr(meta) * constant_from!(8)
                                + store_inner_pos[1].expr(meta) * constant_from!(16)
                                + store_inner_pos[2].expr(meta) * constant_from!(32),
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
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(1),
            move |meta| is_i32.expr(meta),
            move |meta| store_value.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_stack_read_pos = allocator.alloc_memory_table_lookup_read_cell(
            "store read pos",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Stack as u64),
            move |meta| sp.expr(meta) + constant_from!(2),
            move |meta| constant_from!(1),
            move |meta| store_base.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_heap_read = allocator.alloc_memory_table_lookup_read_cell(
            "store load origin",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Heap as u64),
            move |meta| store_block_index.expr(meta),
            move |meta| constant_from!(0),
            move |meta| load_value_in_heap.expr(meta),
            move |meta| constant_from!(1),
        );

        let memory_table_lookup_heap_write = allocator.alloc_memory_table_lookup_write_cell(
            "store write res",
            constraint_builder,
            eid,
            move |meta| constant_from!(LocationType::Heap as u64),
            move |meta| store_block_index.expr(meta),
            move |meta| constant_from!(0),
            move |meta| res.expr(meta),
            move |meta| constant_from!(1),
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
            store_block_index,
            store_inner_pos,
            load_value_in_heap,
            load_value_picked,
            store_value,
            store_value_wrapped,
            store_value_modulus,
            res,
            store_base,
            is_one_byte,
            is_two_bytes,
            is_four_bytes,
            is_eight_bytes,
            is_i32,
            memory_table_lookup_stack_read_val,
            memory_table_lookup_stack_read_pos,
            memory_table_lookup_heap_read,
            memory_table_lookup_heap_write,
            address_within_allocated_pages_helper,
            value_leading_u16,
            value_leading_u16_u8_high,
            value_leading_u16_u8_low,
            store_value_tailing_u16_u8_high,
            store_value_tailing_u16_u8_low,
            lookup_pow,
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
                let byte_index = effective_address as u64;
                let inner_byte_index = byte_index & 7;
                let inner_u16_index = inner_byte_index >> 1;

                let value_leading_u16 = if len == 1 {
                    (pre_block_value1 >> (inner_u16_index * 16)) & 0xffff
                } else {
                    (pre_block_value1 >> (inner_u16_index * 16 + (len - 2) * 8)) & 0xffff
                };
                self.value_leading_u16
                    .assign(ctx, value_leading_u16.into())?;
                self.value_leading_u16_u8_high
                    .assign(ctx, (value_leading_u16 >> 8).into())?;
                self.value_leading_u16_u8_low
                    .assign(ctx, (value_leading_u16 & 0xff).into())?;

                self.store_block_index
                    .assign(ctx, (effective_address as u64) >> 3)?;
                self.store_inner_pos[0].assign_bool(ctx, effective_address & 1 != 0)?;
                self.store_inner_pos[1].assign_bool(ctx, effective_address & 2 != 0)?;
                self.store_inner_pos[2].assign_bool(ctx, effective_address & 4 != 0)?;

                self.store_value_tailing_u16_u8_high
                    .assign(ctx, ((value & 0xffff) >> 8).into())?;
                self.store_value_tailing_u16_u8_low
                    .assign(ctx, (value & 0xff).into())?;

                self.load_value_in_heap.assign(ctx, pre_block_value1)?;

                println!("{} {} {}", effective_address, inner_byte_index, len);
                assert!(inner_byte_index == 0 || len != 8);
                assert!(inner_byte_index & 3 == 0 || len != 4);
                assert!(inner_byte_index & 1 == 0 || len != 2);

                let value_picked = pre_block_value1 >> (inner_byte_index * 8);

                let value_picked = if len == 8 {
                    value_picked
                } else {
                    value_picked & ((1 << (len * 8)) - 1)
                };
                self.load_value_picked.assign(ctx, value_picked.into())?;

                self.store_value.assign(ctx, value)?;
                let value_wrapped = if len == 8 {
                    value
                } else {
                    value & ((1 << (len * 8)) - 1)
                };
                self.store_value_wrapped.assign(ctx, value_wrapped)?;
                self.store_value_modulus
                    .assign(ctx, 1 << (inner_byte_index * 8))?;

                self.res.assign(ctx, updated_block_value1.into())?;

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

                self.store_base.assign(ctx, raw_address as u64)?;

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

                self.memory_table_lookup_heap_read.assign(
                    ctx,
                    entry.memory_rw_entires[2].start_eid,
                    step.current.eid,
                    entry.memory_rw_entires[2].end_eid,
                    effective_address >> 3,
                    LocationType::Heap,
                    false,
                    pre_block_value1,
                )?;

                self.memory_table_lookup_heap_write.assign(
                    ctx,
                    step.current.eid,
                    entry.memory_rw_entires[3].end_eid,
                    effective_address >> 3,
                    LocationType::Heap,
                    false,
                    updated_block_value1,
                )?;
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(1))
    }

    fn memory_writing_ops(&self, entry: &EventTableEntry) -> u32 {
        1
    }
}
