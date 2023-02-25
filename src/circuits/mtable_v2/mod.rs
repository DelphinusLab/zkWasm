use self::allocator::*;
use super::{
    cell::*, config::max_mtable_rows, imtable::InitMemoryTableConfig, rtable::RangeTableConfig,
    test_circuit::v2::IMTABLE_COLUMNS, traits::ConfigureLookupTable, CircuitConfigure,
};
use crate::{constant_from, fixed_curr};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, Fixed, VirtualCells},
};
use specs::{encode::memory_table::encode_memory_table_entry_v2, mtable::LocationType};

mod allocator;
mod assign;

pub(crate) const MEMORY_TABLE_ENTRY_ROWS: i32 = 4;

#[derive(Clone)]
pub struct MemoryTableConfig<F: FieldExt> {
    entry_sel: Column<Fixed>,

    enabled_cell: AllocatedBitCell<F>,
    is_stack_cell: AllocatedBitCell<F>,
    is_heap_cell: AllocatedBitCell<F>,
    is_global_cell: AllocatedBitCell<F>,
    is_next_same_ltype_cell: AllocatedBitCell<F>,
    is_next_same_offset_cell: AllocatedBitCell<F>,
    is_mutable: AllocatedBitCell<F>,

    is_i32_cell: AllocatedBitCell<F>,
    is_i64_cell: AllocatedBitCell<F>,
    is_init_cell: AllocatedBitCell<F>,
    is_imtable_init_cell: AllocatedBitCell<F>,

    start_eid_cell: AllocatedCommonRangeCell<F>,
    end_eid_cell: AllocatedCommonRangeCell<F>,
    eid_diff_cell: AllocatedCommonRangeCell<F>,
    rest_mops_cell: AllocatedCommonRangeCell<F>,
    zero_init_proof_cell: AllocatedCommonRangeCell<F>,
    offset_cell: AllocatedCommonRangeCell<F>,
    offset_diff_cell: AllocatedCommonRangeCell<F>,

    offset_diff_inv_cell: AllocatedUnlimitedCell<F>,
    encode_cell: AllocatedUnlimitedCell<F>,

    value: AllocatedU64Cell<F>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F, IMTABLE_COLUMNS>,
        configure: &CircuitConfigure,
    ) -> Self {
        let entry_sel = meta.fixed_column();

        let mut allocator = MemoryTableCellAllocator::new(meta, entry_sel, rtable, cols);
        allocator.enable_equality(meta, &MemoryTableCellType::CommonRange);

        let enabled_cell = allocator.alloc_bit_cell();
        let is_stack_cell = allocator.alloc_bit_cell();
        let is_heap_cell = allocator.alloc_bit_cell();
        let is_global_cell = allocator.alloc_bit_cell();
        let is_next_same_ltype_cell = allocator.alloc_bit_cell();
        let is_next_same_offset_cell = allocator.alloc_bit_cell();
        let is_mutable = allocator.alloc_bit_cell();

        let is_i32_cell = allocator.alloc_bit_cell();
        let is_i64_cell = allocator.alloc_bit_cell();
        let is_init_cell = allocator.alloc_bit_cell();
        let is_imtable_init_cell = allocator.alloc_bit_cell();

        let start_eid_cell = allocator.alloc_common_range_cell();
        let end_eid_cell = allocator.alloc_common_range_cell();
        let eid_diff_cell = allocator.alloc_common_range_cell();
        let rest_mops_cell = allocator.alloc_common_range_cell();
        let zero_init_proof_cell = allocator.alloc_common_range_cell();
        let offset_cell = allocator.alloc_common_range_cell();
        let offset_diff_cell = allocator.alloc_common_range_cell();

        let offset_diff_inv_cell = allocator.alloc_unlimited_cell();
        let encode_cell = allocator.alloc_unlimited_cell();

        let value = allocator.alloc_u64_cell();

        meta.create_gate("mc1. enable seq", |meta| {
            vec![
                (enabled_cell.curr_expr(meta) - constant_from!(1))
                    * (enabled_cell.next_expr(meta))
                    * fixed_curr!(meta, entry_sel),
            ]
        });

        meta.create_gate("mc2. ltype unique", |meta| {
            vec![
                is_global_cell.curr_expr(meta)
                    + is_heap_cell.curr_expr(meta)
                    + is_stack_cell.curr_expr(meta)
                    - constant_from!(1),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc3. ltype group", |meta| {
            vec![
                (is_stack_cell.curr_expr(meta) - constant_from!(1)) * is_stack_cell.next_expr(meta),
                (is_heap_cell.curr_expr(meta) - constant_from!(1))
                    * is_heap_cell.next_expr(meta)
                    * (is_stack_cell.curr_expr(meta) - constant_from!(1)),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc4a. is_next_same_ltype", |meta| {
            vec![
                is_next_same_ltype_cell.curr_expr(meta)
                    - is_stack_cell.curr_expr(meta) * is_stack_cell.next_expr(meta)
                    - is_global_cell.curr_expr(meta) * is_global_cell.next_expr(meta)
                    - is_heap_cell.curr_expr(meta) * is_heap_cell.next_expr(meta),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc4b. is_next_same_offset", |meta| {
            vec![
                is_next_same_offset_cell.curr_expr(meta)
                    * (is_next_same_ltype_cell.curr_expr(meta) - constant_from!(1)),
                is_next_same_offset_cell.curr_expr(meta) * (offset_diff_cell.curr_expr(meta)),
                (is_next_same_offset_cell.curr_expr(meta) - constant_from!(1))
                    * is_next_same_ltype_cell.curr_expr(meta)
                    * (offset_diff_cell.curr_expr(meta) * offset_diff_inv_cell.curr_expr(meta)
                        - constant_from!(1)),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc5. offset sort", |meta| {
            vec![
                (offset_cell.curr_expr(meta) + offset_diff_cell.curr_expr(meta)
                    - offset_cell.next_expr(meta))
                    * is_next_same_ltype_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc6. eid sort", |meta| {
            vec![
                (start_eid_cell.curr_expr(meta)
                    + eid_diff_cell.curr_expr(meta)
                    + constant_from!(1)
                    - end_eid_cell.curr_expr(meta)),
                (end_eid_cell.curr_expr(meta) - start_eid_cell.next_expr(meta))
                    * is_next_same_offset_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc7a. init", |meta| {
            vec![
                is_init_cell.curr_expr(meta) * start_eid_cell.curr_expr(meta),
                is_init_cell.curr_expr(meta)
                    * (is_imtable_init_cell.curr_expr(meta) - constant_from!(1))
                    * (is_heap_cell.curr_expr(meta) - constant_from!(1)),
                is_init_cell.curr_expr(meta)
                    * (is_imtable_init_cell.curr_expr(meta) - constant_from!(1))
                    * (offset_cell.curr_expr(meta)
                        - zero_init_proof_cell.curr_expr(meta)
                        - constant_from!(configure.first_consecutive_zero_memory_offset)),
                is_init_cell.curr_expr(meta)
                    * (is_imtable_init_cell.curr_expr(meta) - constant_from!(1))
                    * value.u64_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate(
            "mc7b. global must has init (because of mutability check).",
            |meta| {
                vec![
                    (is_next_same_offset_cell.expr(meta) - constant_from!(1))
                        * is_global_cell.next_expr(meta)
                        * (is_init_cell.next_expr(meta) - constant_from!(1)),
                ]
                .into_iter()
                .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
                .collect::<Vec<_>>()
            },
        );

        imtable.configure_in_table(
            meta,
            "mc7c. imtable init",
            |meta| {
                is_init_cell.curr_expr(meta)
                    * is_imtable_init_cell.curr_expr(meta)
                    * imtable.encode(
                        is_mutable.curr_expr(meta),
                        is_stack_cell.curr_expr(meta) * constant_from!(LocationType::Stack as u64)
                            + is_heap_cell.curr_expr(meta)
                                * constant_from!(LocationType::Heap as u64)
                            + is_global_cell.curr_expr(meta)
                                * constant_from!(LocationType::Global as u64),
                        offset_cell.curr_expr(meta),
                        value.u64_cell.curr_expr(meta),
                    )
                    * enabled_cell.curr_expr(meta)
                    * fixed_curr!(meta, entry_sel)
            },
            0,
        );

        meta.create_gate("mc8. vtype", |meta| {
            vec![
                is_i32_cell.curr_expr(meta) + is_i64_cell.curr_expr(meta) - constant_from!(1),
                is_heap_cell.curr_expr(meta) * is_i32_cell.curr_expr(meta),
                is_i32_cell.curr_expr(meta)
                    * (value.u16_cells_le[2].curr_expr(meta)
                        + value.u16_cells_le[3].curr_expr(meta)),
                is_global_cell.curr_expr(meta)
                    * is_next_same_offset_cell.curr_expr(meta)
                    * (is_i32_cell.curr_expr(meta) - is_i32_cell.next_expr(meta)),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        /* mc9. value is in allocator */

        meta.create_gate("mc10b. rest_mops", |meta| {
            vec![
                is_init_cell.curr_expr(meta)
                    * (rest_mops_cell.next_expr(meta) - rest_mops_cell.curr_expr(meta)),
                (is_init_cell.curr_expr(meta) - constant_from!(1))
                    * (rest_mops_cell.next_expr(meta) + constant_from!(1)
                        - rest_mops_cell.curr_expr(meta)),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc10c. rest_mops", |meta| {
            vec![
                (enabled_cell.curr_expr(meta) - constant_from!(1)) * rest_mops_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc11. mutable", |meta| {
            vec![
                (is_init_cell.curr_expr(meta) - constant_from!(1))
                    * (is_mutable.curr_expr(meta) - constant_from!(1)),
                (is_mutable.curr_expr(meta) - is_mutable.next_expr(meta))
                    * is_next_same_offset_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc12. lookup encode", |meta| {
            vec![
                encode_memory_table_entry_v2(
                    start_eid_cell.curr_expr(meta),
                    end_eid_cell.curr_expr(meta),
                    offset_cell.curr_expr(meta),
                    is_stack_cell.curr_expr(meta) * constant_from!(LocationType::Stack as u64)
                        + is_global_cell.curr_expr(meta)
                            * constant_from!(LocationType::Global as u64)
                        + is_heap_cell.curr_expr(meta) * constant_from!(LocationType::Heap),
                    is_i32_cell.curr_expr(meta),
                    value.u64_cell.curr_expr(meta),
                ) - encode_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * enabled_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        Self {
            entry_sel,
            enabled_cell,
            is_stack_cell,
            is_heap_cell,
            is_global_cell,
            is_next_same_ltype_cell,
            is_next_same_offset_cell,
            is_mutable,
            is_i32_cell,
            is_i64_cell,
            is_init_cell,
            is_imtable_init_cell,
            start_eid_cell,
            end_eid_cell,
            eid_diff_cell,
            rest_mops_cell,
            zero_init_proof_cell,
            offset_cell,
            offset_diff_cell,
            offset_diff_inv_cell,
            value,
            encode_cell,
        }
    }
}

impl<F: FieldExt> ConfigureLookupTable<F> for MemoryTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any("etable mtable lookup", |meta| {
            vec![(expr(meta), self.encode_cell.expr(meta))]
        });
    }
}

pub(super) struct MemoryTableChip<F: FieldExt> {
    config: MemoryTableConfig<F>,
    maximal_available_rows: usize,
}

impl<F: FieldExt> MemoryTableChip<F> {
    pub(super) fn new(config: MemoryTableConfig<F>) -> Self {
        Self {
            config,
            maximal_available_rows: max_mtable_rows() as usize / MEMORY_TABLE_ENTRY_ROWS as usize
                * MEMORY_TABLE_ENTRY_ROWS as usize,
        }
    }
}
