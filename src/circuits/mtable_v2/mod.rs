use self::allocator::*;
use super::imtable::InitMemoryTableConfig;
use super::{cell::*, rtable::RangeTableConfig, CircuitConfigure};
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::{Advice, Column, ConstraintSystem, Fixed};

mod allocator;

pub(crate) const MSTEP_SIZE: i32 = 4;

pub struct MemoryTableConfig<F: FieldExt> {
    step_sel: Column<Fixed>,
    enable_cell: AllocatedBitCell<F>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut (impl Iterator<Item = Column<Advice>> + Clone),
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
        configure: &CircuitConfigure,
    ) -> Self {
        let sel = meta.fixed_column();
        let step_sel = meta.fixed_column();

        let mut allocator = MemoryTableCellAllocator::new(meta, rtable, cols);
        allocator.enable_equality(meta, &MemoryTableCellType::CommonRange);

        let enable_cell = allocator.alloc_bit_cell();
        let is_stack_cell = allocator.alloc_bit_cell();
        let is_heap_cell = allocator.alloc_bit_cell();
        let is_global_cell = allocator.alloc_bit_cell();
        let is_next_same_ltype_cell = allocator.alloc_bit_cell();
        let is_next_same_offset_cell = allocator.alloc_bit_cell();

        let is_i32_cell = allocator.alloc_bit_cell();
        let is_i64_cell = allocator.alloc_bit_cell();
        let is_init_cell = allocator.alloc_bit_cell();
        let is_zero_init_cell = allocator.alloc_bit_cell();

        let start_eid_cell = allocator.alloc_common_range_cell();
        let end_eid_cell = allocator.alloc_common_range_cell();
        let eid_diff_cell = allocator.alloc_common_range_cell();
        let rest_mops_cell = allocator.alloc_common_range_cell();
        let zero_init_proof_cell = allocator.alloc_common_range_cell();
        let offset_cell = allocator.alloc_common_range_cell();
        let offset_diff = allocator.alloc_common_range_cell();

        let value = allocator.alloc_u64_cell();

        Self {
            step_sel,
            enable_cell,
        }
    }
}
