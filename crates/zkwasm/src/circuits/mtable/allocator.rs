use std::collections::BTreeMap;
use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;

use crate::circuits::cell::*;
use crate::circuits::rtable::RangeTableConfig;
use crate::circuits::utils::bit::BitColumn;
use crate::circuits::utils::common_range::CommonRangeColumn;
use crate::circuits::utils::u16::U16Column;
use crate::constant_from;
use crate::fixed_curr;
use crate::nextn;

use super::MEMORY_TABLE_ENTRY_ROWS;

pub(super) trait MemoryTableCellExpression<F: FieldExt> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
}

impl<F: FieldExt> MemoryTableCellExpression<F> for AllocatedCell<F> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot + MEMORY_TABLE_ENTRY_ROWS)
    }

    fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot - MEMORY_TABLE_ENTRY_ROWS)
    }
}

macro_rules! impl_cell {
    ($x: ident) => {
        impl<F: FieldExt> MemoryTableCellExpression<F> for $x<F> {
            fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
                self.cell.next_expr(meta)
            }

            fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
                self.cell.prev_expr(meta)
            }
        }
    };
}

impl_cell!(AllocatedBitCell);
impl_cell!(AllocatedCommonRangeCell);
impl_cell!(AllocatedU16Cell);
impl_cell!(AllocatedUnlimitedCell);

impl<F: FieldExt> MemoryTableCellExpression<F> for AllocatedU32Cell<F> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.u16_cells_le[0].next_expr(meta)
            + (self.u16_cells_le[1].next_expr(meta) * constant_from!(1 << 16))
    }

    fn prev_expr(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MemoryTableCellType {
    Bit = 1,
    U16,
    CommonRange,
    Unlimited,
}

const BIT_COLUMNS: usize = 3;
const U16_COLUMNS: usize = U32_CELLS.next_multiple_of(2) / 2 + U64_CELLS;
const COMMON_RANGE_COLUMNS: usize = 1;
const UNLIMITED_COLUMNS: usize = 2;
const U32_CELLS: usize = if cfg!(feature = "continuation") { 5 } else { 2 };
const U64_CELLS: usize = 1;

#[derive(Debug, Clone)]
pub(super) struct MemoryTableCellAllocator<F: FieldExt> {
    all_cols: BTreeMap<MemoryTableCellType, Vec<Column<Advice>>>,
    free_cells: BTreeMap<MemoryTableCellType, (usize, u32)>,
    free_u32_cells: Vec<AllocatedU32Cell<F>>,
    free_u64_cells: Vec<AllocatedU64Cell<F>>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> MemoryTableCellAllocator<F> {
    pub fn assert_no_free_cells(&self) {
        for (t, (i, j)) in &self.free_cells {
            let cols = self.all_cols.get(t).unwrap();

            assert!(
                *i == cols.len() || (*i == cols.len() - 1 && *j > 0),
                "unused {:?} col should be removed: {}.",
                t,
                cols.len() - *i - (*j != 0) as usize
            );
        }

        assert!(
            self.free_u32_cells.is_empty(),
            "unused u32 cells should be removed: {:?}.",
            self.free_u32_cells.len()
        );
        assert!(
            self.free_u64_cells.is_empty(),
            "unused u64 cells should be removed: {:?}.",
            self.free_u64_cells.len()
        );
    }

    pub fn enable_equality(&mut self, meta: &mut ConstraintSystem<F>, t: &MemoryTableCellType) {
        for c in self.all_cols.get(t).unwrap() {
            meta.enable_equality(*c);
        }
    }

    pub(super) fn prepare_alloc_u32_cell(&mut self) -> AllocatedU32Cell<F> {
        let u16_cells_le = [0; 2].map(|_| self.alloc_u16_cell());

        AllocatedU32Cell { u16_cells_le }
    }

    pub(super) fn prepare_alloc_u64_cell(
        &mut self,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> AllocatedU64Cell<F> {
        let u16_cells_le = [0; 4].map(|_| self.alloc_u16_cell());
        let u64_cell = self.alloc_unlimited_cell();
        meta.create_gate("mc9. value", |meta| {
            let init = u64_cell.curr_expr(meta);
            vec![
                (0..4)
                    .map(|x| u16_cells_le[x].curr_expr(meta) * constant_from!(1u64 << (16 * x)))
                    .fold(init, |acc, x| acc - x)
                    * enable(meta),
            ]
        });
        AllocatedU64Cell {
            u16_cells_le,
            u64_cell,
        }
    }

    pub(super) fn new(
        meta: &mut ConstraintSystem<F>,
        sel: Column<Fixed>,
        (l_0, l_active, l_active_last): (Column<Fixed>, Column<Fixed>, Column<Fixed>),
        rtable: &RangeTableConfig<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut allocator = Self::_new(meta, sel, (l_0, l_active, l_active_last), rtable, cols);
        for _ in 0..U32_CELLS {
            let cell = allocator.prepare_alloc_u32_cell();
            allocator.free_u32_cells.push(cell);
        }
        for _ in 0..U64_CELLS {
            let cell = allocator.prepare_alloc_u64_cell(meta, |meta| fixed_curr!(meta, sel));
            allocator.free_u64_cells.push(cell);
        }
        allocator
    }

    fn _new(
        meta: &mut ConstraintSystem<F>,
        sel: Column<Fixed>,
        (l_0, l_active, l_active_last): (Column<Fixed>, Column<Fixed>, Column<Fixed>),
        rtable: &RangeTableConfig<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut all_cols = BTreeMap::new();
        all_cols.insert(
            MemoryTableCellType::Bit,
            [0; BIT_COLUMNS]
                .map(|_| BitColumn::configure(meta, cols, |meta| fixed_curr!(meta, sel)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            MemoryTableCellType::U16,
            [0; U16_COLUMNS]
                .map(|_| U16Column::configure(meta, (l_0, l_active, l_active_last)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            MemoryTableCellType::CommonRange,
            [0; COMMON_RANGE_COLUMNS]
                .map(|_| {
                    CommonRangeColumn::configure(meta, cols, rtable, |_| constant_from!(1)).col
                })
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            MemoryTableCellType::Unlimited,
            [0; UNLIMITED_COLUMNS]
                .map(|_| cols.next().unwrap())
                .into_iter()
                .collect(),
        );
        Self {
            all_cols,
            free_cells: BTreeMap::from_iter(
                vec![
                    (MemoryTableCellType::Bit, (0, 0)),
                    (MemoryTableCellType::U16, (0, 0)),
                    (MemoryTableCellType::CommonRange, (0, 0)),
                    (MemoryTableCellType::Unlimited, (0, 0)),
                ]
                .into_iter(),
            ),
            free_u32_cells: vec![],
            free_u64_cells: vec![],
            _mark: PhantomData,
        }
    }

    fn alloc(&mut self, t: &MemoryTableCellType) -> AllocatedCell<F> {
        let v = self.free_cells.get_mut(t).unwrap();
        let res = AllocatedCell {
            col: self.all_cols.get(t).unwrap()[v.0],
            rot: v.1 as i32,
            _mark: PhantomData,
        };

        v.1 += 1;
        if v.1 == MEMORY_TABLE_ENTRY_ROWS as u32 {
            v.0 += 1;
            v.1 = 0;
        }

        res
    }

    pub(super) fn alloc_bit_cell(&mut self) -> AllocatedBitCell<F> {
        AllocatedBitCell {
            cell: self.alloc(&MemoryTableCellType::Bit),
        }
    }

    pub(super) fn alloc_common_range_cell(&mut self) -> AllocatedCommonRangeCell<F> {
        AllocatedCommonRangeCell {
            cell: self.alloc(&MemoryTableCellType::CommonRange),
        }
    }

    #[cfg(feature = "continuation")]
    pub(super) fn alloc_u32_state_cell(&mut self) -> AllocatedU32Cell<F> {
        self.alloc_u32_cell()
    }

    #[cfg(not(feature = "continuation"))]
    pub(super) fn alloc_u32_state_cell(&mut self) -> AllocatedCommonRangeCell<F> {
        self.alloc_common_range_cell()
    }

    pub(super) fn alloc_u16_cell(&mut self) -> AllocatedU16Cell<F> {
        AllocatedU16Cell {
            cell: self.alloc(&MemoryTableCellType::U16),
        }
    }

    pub(super) fn alloc_unlimited_cell(&mut self) -> AllocatedUnlimitedCell<F> {
        AllocatedUnlimitedCell {
            cell: self.alloc(&MemoryTableCellType::Unlimited),
        }
    }

    pub(super) fn alloc_u32_cell(&mut self) -> AllocatedU32Cell<F> {
        self.free_u32_cells.pop().expect("no more free u32 cells")
    }

    pub(super) fn alloc_u64_cell(&mut self) -> AllocatedU64Cell<F> {
        self.free_u64_cells.pop().expect("no more free u64 cells")
    }
}
