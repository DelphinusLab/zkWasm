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

pub(crate) mod utils;

pub(crate) const MEMORY_TABLE_ENTRY_ROWS: i32 = 4;

#[cfg(feature = "continuation")]
type AllocatedU32StateCell<F> = AllocatedU32Cell<F>;
#[cfg(not(feature = "continuation"))]
type AllocatedU32StateCell<F> = AllocatedCommonRangeCell<F>;

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

    start_eid_cell: AllocatedU32StateCell<F>,
    end_eid_cell: AllocatedU32StateCell<F>,
    eid_diff_cell: AllocatedU32StateCell<F>,
    rest_mops_cell: AllocatedCommonRangeCell<F>,
    offset_cell: AllocatedU32Cell<F>,
    offset_diff_cell: AllocatedU32Cell<F>,

    offset_diff_inv_cell: AllocatedUnlimitedCell<F>,
    offset_diff_inv_helper_cell: AllocatedUnlimitedCell<F>,
    encode_cell: AllocatedUnlimitedCell<F>,
    init_encode_cell: AllocatedUnlimitedCell<F>,

    #[cfg(feature = "continuation")]
    address_encode_cell: AllocatedUnlimitedCell<F>,
    #[cfg(feature = "continuation")]
    post_init_encode_cell: AllocatedUnlimitedCell<F>,
    #[cfg(feature = "continuation")]
    rest_memory_finalize_ops_cell: AllocatedUnlimitedCell<F>,

    value: AllocatedU64Cell<F>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        (l_0, l_active, l_active_last): (Column<Fixed>, Column<Fixed>, Column<Fixed>),
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        rtable: &RangeTableConfig<F>,
        image_table: &ImageTableConfig<F>,
    ) -> Self {
        let entry_sel = meta.fixed_column();

        let mut allocator = MemoryTableCellAllocator::new(
            meta,
            entry_sel,
            (l_0, l_active, l_active_last),
            rtable,
            cols,
        );
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

        let start_eid_cell = allocator.alloc_u32_state_cell();
        let end_eid_cell = allocator.alloc_u32_state_cell();
        let eid_diff_cell = allocator.alloc_u32_state_cell();
        let rest_mops_cell = allocator.alloc_common_range_cell();

        let offset_cell = allocator.alloc_u32_cell();

        let offset_diff_cell = allocator.alloc_u32_cell();
        let offset_diff_inv_cell = allocator.alloc_unlimited_cell();
        let offset_diff_inv_helper_cell = allocator.alloc_unlimited_cell();
        let encode_cell = allocator.alloc_unlimited_cell();
        let init_encode_cell = allocator.alloc_unlimited_cell();

        #[cfg(feature = "continuation")]
        let post_init_encode_cell = allocator.alloc_unlimited_cell();
        #[cfg(feature = "continuation")]
        let address_encode_cell = allocator.alloc_unlimited_cell();

        #[cfg(feature = "continuation")]
        let rest_memory_finalize_ops_cell = {
            let cell = allocator.alloc_unlimited_cell();
            // FIXME: try to avoid this?
            meta.enable_equality(cell.cell.col);
            cell
        };

        let value = allocator.alloc_u64_cell();

        allocator.assert_no_free_cells();

        macro_rules! location {
            ($meta:expr) => {
                is_stack_cell.curr_expr($meta) * constant_from!(LocationType::Stack as u64)
                    + is_global_cell.curr_expr($meta) * constant_from!(LocationType::Global as u64)
                    + is_heap_cell.curr_expr($meta) * constant_from!(LocationType::Heap)
            };
        }
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

        meta.create_gate(
            "mc7a. global must has init (because of mutability check).",
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

        meta.create_gate("mc7b. init encode.", |meta| {
            vec![
                (encode_init_memory_table_entry(
                    location!(meta),
                    offset_cell.curr_expr(meta),
                    is_mutable.curr_expr(meta),
                    start_eid_cell.curr_expr(meta),
                    value.u64_cell.curr_expr(meta),
                ) - init_encode_cell.curr_expr(meta))
                    * is_init_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        image_table.init_memory_lookup(meta, "mc7c. imtable init", |meta| {
            cfg_if::cfg_if! {
                if #[cfg(feature = "continuation")] {
                    (
                        address_encode_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel),
                        init_encode_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel),
                    )
                } else {
                    init_encode_cell.curr_expr(meta) * fixed_curr!(meta, entry_sel)
                }
            }
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
                    location!(meta),
                    is_i32_cell.curr_expr(meta),
                ) - encode_cell.curr_expr(meta),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, entry_sel))
            .collect::<Vec<_>>()
        });

        #[cfg(feature = "continuation")]
        {
            use specs::encode::init_memory_table::encode_init_memory_table_address;
            use specs::encode::init_memory_table::MEMORY_ADDRESS_OFFSET;

            meta.create_gate("mc13. post init memory entry", |meta| {
                let is_writing = constant_from!(1) - is_init_cell.curr_expr(meta);
                let next_entry_at_different_position =
                    constant_from!(1) - is_next_same_offset_cell.curr_expr(meta);

                let is_memory_finalized_position_bit =
                    is_writing * next_entry_at_different_position;

                vec![
                    // rest_memory_finalize_ops_cell decreases.
                    // `* enabled_cell`: If disabled, rest_memory_finalize_ops_cell should keep the same,
                    // The termination rest_memory_finalize_ops_cell is constant 0 at the last selected(sel=1) step.
                    rest_memory_finalize_ops_cell.curr_expr(meta)
                        - rest_memory_finalize_ops_cell.next_expr(meta)
                        - is_memory_finalized_position_bit.clone() * enabled_cell.curr_expr(meta),
                    // encode address_encode_cell.
                    (encode_init_memory_table_address(
                        location!(meta),
                        offset_cell.curr_expr(meta),
                    ) - address_encode_cell.curr_expr(meta)),
                    // post_init_encode_cell assigned iff at memory finalized position.
                    post_init_encode_cell.curr_expr(meta)
                        * (constant_from!(1) - is_memory_finalized_position_bit.clone()),
                    post_init_encode_cell.curr_expr(meta)
                        * (constant_from!(1) - enabled_cell.curr_expr(meta)),
                    // encode post_init_encode_cell.
                    (post_init_encode_cell.curr_expr(meta)
                        - address_encode_cell.curr_expr(meta)
                            * constant_from!(MEMORY_ADDRESS_OFFSET)
                        - encode_init_memory_table_entry(
                            location!(meta),
                            offset_cell.curr_expr(meta),
                            is_mutable.curr_expr(meta),
                            start_eid_cell.curr_expr(meta),
                            value.u64_cell.curr_expr(meta),
                        ))
                        * is_memory_finalized_position_bit,
                ]
                .into_iter()
                .map(|x| x * fixed_curr!(meta, entry_sel))
                .collect::<Vec<_>>()
            });
        }

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
            value,
            init_encode_cell,
            encode_cell,

            #[cfg(feature = "continuation")]
            post_init_encode_cell,
            #[cfg(feature = "continuation")]
            address_encode_cell,
            #[cfg(feature = "continuation")]
            rest_memory_finalize_ops_cell,
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

#[cfg(feature = "continuation")]
impl<F: FieldExt> MemoryTableConfig<F> {
    pub(in crate::circuits) fn configure_in_post_init_memory_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        name: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        use specs::encode::image_table::ImageTableEncoder;

        meta.lookup_any(name, |meta| {
            let encode = expr(meta);
            vec![(
                encode,
                ImageTableEncoder::InitMemory.encode(self.post_init_encode_cell.expr(meta))
                    * fixed_curr!(meta, self.entry_sel),
            )]
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
