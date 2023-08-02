use self::allocator::*;
use super::cell::*;
use super::image_table::ImageTableConfig;
use super::rtable::RangeTableConfig;
use super::traits::ConfigureLookupTable;
use crate::constant_from;
use crate::fixed_curr;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::init_memory_table::encode_init_memory_table_entry;
use specs::encode::memory_table::encode_memory_table_entry;
use specs::mtable::LocationType;

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

    start_eid_cell: AllocatedCommonRangeCell<F>,
    end_eid_cell: AllocatedCommonRangeCell<F>,
    eid_diff_cell: AllocatedCommonRangeCell<F>,
    rest_mops_cell: AllocatedCommonRangeCell<F>,
    offset_align_left: AllocatedCommonRangeCell<F>,
    offset_align_right: AllocatedCommonRangeCell<F>,
    offset_align_left_diff_cell: AllocatedCommonRangeCell<F>,
    offset_align_right_diff_cell: AllocatedCommonRangeCell<F>,
    offset_cell: AllocatedCommonRangeCell<F>,
    offset_diff_cell: AllocatedCommonRangeCell<F>,

    offset_diff_inv_cell: AllocatedUnlimitedCell<F>,
    offset_diff_inv_helper_cell: AllocatedUnlimitedCell<F>,
    encode_cell: AllocatedUnlimitedCell<F>,
    init_encode_cell: AllocatedUnlimitedCell<F>,

    value: AllocatedU64Cell<F>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        rtable: &RangeTableConfig<F>,
        image_table: &ImageTableConfig<F>,
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

        let start_eid_cell = allocator.alloc_common_range_cell();
        let end_eid_cell = allocator.alloc_common_range_cell();
        let eid_diff_cell = allocator.alloc_common_range_cell();
        let rest_mops_cell = allocator.alloc_common_range_cell();

        let offset_align_left = allocator.alloc_common_range_cell();
        let offset_align_right = allocator.alloc_common_range_cell();
        let offset_cell = allocator.alloc_common_range_cell();
        let offset_align_left_diff_cell = allocator.alloc_common_range_cell();
        let offset_align_right_diff_cell = allocator.alloc_common_range_cell();

        let offset_diff_cell = allocator.alloc_common_range_cell();
        let offset_diff_inv_cell = allocator.alloc_unlimited_cell();
        let offset_diff_inv_helper_cell = allocator.alloc_unlimited_cell();
        let encode_cell = allocator.alloc_unlimited_cell();
        let init_encode_cell = allocator.alloc_unlimited_cell();

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
                    - enabled_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
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
            .map(|x| x * fixed_curr!(meta, entry_sel))
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
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc4b. is_next_same_offset", |meta| {
            vec![
                is_next_same_offset_cell.curr_expr(meta)
                    * (is_next_same_ltype_cell.curr_expr(meta) - constant_from!(1)),
                is_next_same_offset_cell.curr_expr(meta) * offset_diff_cell.curr_expr(meta),
                offset_diff_cell.curr_expr(meta) * offset_diff_inv_cell.curr_expr(meta)
                    - offset_diff_inv_helper_cell.curr_expr(meta),
                (is_next_same_offset_cell.curr_expr(meta) - constant_from!(1))
                    * is_next_same_ltype_cell.curr_expr(meta)
                    * (offset_diff_inv_helper_cell.curr_expr(meta) - constant_from!(1)),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc5. offset sort", |meta| {
            vec![
                (offset_cell.curr_expr(meta) + offset_diff_cell.curr_expr(meta)
                    - offset_cell.next_expr(meta))
                    * is_next_same_ltype_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
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
                // offset_left_align <= offset && offset <= offset_right_align
                is_init_cell.curr_expr(meta)
                    * (offset_align_left.curr_expr(meta)
                        + offset_align_left_diff_cell.curr_expr(meta)
                        - offset_cell.curr_expr(meta)),
                is_init_cell.curr_expr(meta)
                    * (offset_cell.curr_expr(meta) + offset_align_right_diff_cell.curr_expr(meta)
                        - offset_align_right.curr_expr(meta)),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
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
                .map(|x| x * fixed_curr!(meta, entry_sel))
                .collect::<Vec<_>>()
            },
        );

        meta.create_gate("mc7c. init encode.", |meta| {
            vec![
                is_init_cell.curr_expr(meta)
                    * encode_init_memory_table_entry(
                        is_stack_cell.curr_expr(meta) * constant_from!(LocationType::Stack as u64)
                            + is_heap_cell.curr_expr(meta)
                                * constant_from!(LocationType::Heap as u64)
                            + is_global_cell.curr_expr(meta)
                                * constant_from!(LocationType::Global as u64),
                        is_mutable.curr_expr(meta),
                        offset_align_left.curr_expr(meta),
                        offset_align_right.curr_expr(meta),
                        value.u64_cell.curr_expr(meta),
                    )
                    - init_encode_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        image_table.init_memory_lookup(meta, "mc7c. imtable init", |meta| {
            init_encode_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel)
        });

        meta.create_gate("mc8. vtype", |meta| {
            vec![
                is_i32_cell.curr_expr(meta) + is_i64_cell.curr_expr(meta)
                    - enabled_cell.curr_expr(meta),
                is_heap_cell.curr_expr(meta) * is_i32_cell.curr_expr(meta),
                is_i32_cell.curr_expr(meta)
                    * (value.u16_cells_le[2].curr_expr(meta)
                        + value.u16_cells_le[3].curr_expr(meta)),
                is_global_cell.curr_expr(meta)
                    * is_next_same_offset_cell.curr_expr(meta)
                    * (is_i32_cell.curr_expr(meta) - is_i32_cell.next_expr(meta)),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        /* mc9. value is in allocator */

        meta.create_gate("mc10b. rest_mops", |meta| {
            vec![
                is_init_cell.curr_expr(meta)
                    * (rest_mops_cell.next_expr(meta) - rest_mops_cell.curr_expr(meta)),
                (is_init_cell.curr_expr(meta) - constant_from!(1))
                    * (rest_mops_cell.next_expr(meta) + enabled_cell.curr_expr(meta)
                        - rest_mops_cell.curr_expr(meta)),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mc10c. rest_mops decrease to zero", |meta| {
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
                (constant_from!(1) - enabled_cell.curr_expr(meta)) * encode_cell.curr_expr(meta),
                encode_memory_table_entry(
                    offset_cell.curr_expr(meta),
                    is_stack_cell.curr_expr(meta) * constant_from!(LocationType::Stack as u64)
                        + is_global_cell.curr_expr(meta)
                            * constant_from!(LocationType::Global as u64)
                        + is_heap_cell.curr_expr(meta) * constant_from!(LocationType::Heap),
                    is_i32_cell.curr_expr(meta),
                ) - encode_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
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
            start_eid_cell,
            end_eid_cell,
            eid_diff_cell,
            rest_mops_cell,
            offset_cell,
            offset_diff_cell,
            offset_diff_inv_cell,
            offset_diff_inv_helper_cell,
            offset_align_left,
            offset_align_right,
            offset_align_left_diff_cell,
            offset_align_right_diff_cell,
            value,
            init_encode_cell,
            encode_cell,
        }
    }
}

impl<F: FieldExt> ConfigureLookupTable<F> for MemoryTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        name: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Vec<Expression<F>>,
    ) {
        meta.lookup_any(name, |meta| {
            let mut expr = expr(meta);
            expr.reverse();
            vec![
                (
                    expr.pop().unwrap(),
                    self.start_eid_cell.expr(meta) * fixed_curr!(meta, self.entry_sel),
                ),
                (
                    expr.pop().unwrap(),
                    self.end_eid_cell.expr(meta) * fixed_curr!(meta, self.entry_sel),
                ),
                (
                    expr.pop().unwrap(),
                    self.encode_cell.expr(meta) * fixed_curr!(meta, self.entry_sel),
                ),
                (
                    expr.pop().unwrap(),
                    self.value.expr(meta) * fixed_curr!(meta, self.entry_sel),
                ),
            ]
        });
    }
}

pub(super) struct MemoryTableChip<F: FieldExt> {
    config: MemoryTableConfig<F>,
    maximal_available_rows: usize,
}

impl<F: FieldExt> MemoryTableChip<F> {
    pub(super) fn new(config: MemoryTableConfig<F>, maximal_available_rows: usize) -> Self {
        Self {
            config,
            maximal_available_rows: maximal_available_rows / MEMORY_TABLE_ENTRY_ROWS as usize
                * MEMORY_TABLE_ENTRY_ROWS as usize,
        }
    }
}
