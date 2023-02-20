use std::{collections::BTreeMap, marker::PhantomData};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};

use crate::{
    circuits::{
        rtable::RangeTableConfig,
        utils::{bit::BitColumn, common_range::CommonRangeColumn, u16::U16Column},
    },
    constant_from, nextn,
};

use super::ESTEP_SIZE;

#[derive(Debug, Clone, Copy)]
pub(super) struct AllocatedCell<F: FieldExt> {
    pub(super) col: Column<Advice>,
    pub(super) rot: i32,
    _mark: PhantomData<F>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AllocatedBitCell<F: FieldExt>(pub(super) AllocatedCell<F>);

#[derive(Debug, Clone, Copy)]
pub(super) struct AllocatedCommonRangeCell<F: FieldExt>(pub(super) AllocatedCell<F>);

#[derive(Debug, Clone, Copy)]
pub(super) struct AllocatedU16Cell<F: FieldExt>(pub(super) AllocatedCell<F>);

#[derive(Debug, Clone, Copy)]
pub(super) struct AllocatedUnlimitedCell<F: FieldExt>(pub(super) AllocatedCell<F>);

pub(super) trait CellExpression<F: FieldExt> {
    fn curr_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn next_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn prev_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
}

impl<F: FieldExt> CellExpression<F> for AllocatedCell<F> {
    fn curr_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }

    fn next_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot + ESTEP_SIZE as i32)
    }

    fn prev_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot - ESTEP_SIZE as i32)
    }
}

impl<F: FieldExt> CellExpression<F> for AllocatedBitCell<F> {
    fn curr_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.curr_expr(meta)
    }

    fn next_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.next_expr(meta)
    }

    fn prev_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.prev_expr(meta)
    }
}

impl<F: FieldExt> CellExpression<F> for AllocatedCommonRangeCell<F> {
    fn curr_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.curr_expr(meta)
    }

    fn next_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.next_expr(meta)
    }

    fn prev_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.prev_expr(meta)
    }
}

impl<F: FieldExt> CellExpression<F> for AllocatedUnlimitedCell<F> {
    fn curr_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.curr_expr(meta)
    }

    fn next_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.next_expr(meta)
    }

    fn prev_expr(self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.0.prev_expr(meta)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ETableCellType {
    Bit = 1,
    U16,
    CommonRange,
    Unlimited,
    MTableLookup,
}

const BIT_COLUMNS: usize = 5;
const U16_COLUMNS: usize = 2;
const COMMON_RANGE_COLUMNS: usize = 3;
const UNLIMITED_COLUMNS: usize = 2;
const MTABLE_LOOKUP_COLUMNS: usize = 1;

#[derive(Debug, Clone)]
pub(super) struct CellAllocator<F: FieldExt> {
    free_cells: BTreeMap<ETableCellType, (usize, u32)>,
    all_cols: BTreeMap<ETableCellType, Vec<Column<Advice>>>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> CellAllocator<F> {
    pub fn enable_equality(&mut self, meta: &mut ConstraintSystem<F>, t: &ETableCellType) {
        for c in self.all_cols.get(t).unwrap() {
            meta.enable_equality(*c);
        }
    }

    pub(super) fn new(
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut all_cols = BTreeMap::new();
        all_cols.insert(
            ETableCellType::Bit,
            [0; BIT_COLUMNS]
                .map(|_| BitColumn::configure(meta, cols, |_| constant_from!(1)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            ETableCellType::U16,
            [0; U16_COLUMNS]
                .map(|_| U16Column::configure(meta, cols, rtable, |_| constant_from!(1)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            ETableCellType::CommonRange,
            [0; COMMON_RANGE_COLUMNS]
                .map(|_| {
                    CommonRangeColumn::configure(meta, cols, rtable, |_| constant_from!(1)).col
                })
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            ETableCellType::Unlimited,
            [0; UNLIMITED_COLUMNS]
                .map(|_| cols.next().unwrap())
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            ETableCellType::MTableLookup,
            [0; MTABLE_LOOKUP_COLUMNS]
                .map(|_| cols.next().unwrap())
                .into_iter()
                .collect(),
        );

        Self {
            all_cols,
            free_cells: BTreeMap::from_iter(
                vec![
                    (ETableCellType::Bit, (0, 0)),
                    (ETableCellType::U16, (0, 0)),
                    (ETableCellType::CommonRange, (0, 0)),
                    (ETableCellType::Unlimited, (0, 0)),
                    (ETableCellType::MTableLookup, (0, 0)),
                ]
                .into_iter(),
            ),
            _mark: PhantomData,
        }
    }

    fn alloc(&mut self, t: &ETableCellType) -> AllocatedCell<F> {
        let v = self.free_cells.get_mut(t).unwrap();
        let res = AllocatedCell {
            col: self.all_cols.get(t).unwrap()[v.0],
            rot: v.1 as i32,
            _mark: PhantomData,
        };

        assert!(v.0 < BIT_COLUMNS);

        v.1 += 1;
        if v.1 == ESTEP_SIZE as u32 {
            v.0 += 1;
            v.1 = 0;
        }

        res
    }

    pub(super) fn alloc_bit_cell(&mut self) -> AllocatedBitCell<F> {
        AllocatedBitCell(self.alloc(&ETableCellType::Bit))
    }

    pub(super) fn alloc_common_range_cell(&mut self) -> AllocatedCommonRangeCell<F> {
        AllocatedCommonRangeCell(self.alloc(&ETableCellType::CommonRange))
    }

    pub(super) fn alloc_u16_cell(&mut self) -> AllocatedU16Cell<F> {
        AllocatedU16Cell(self.alloc(&ETableCellType::U16))
    }

    pub(super) fn alloc_unlimited_cell(&mut self) -> AllocatedUnlimitedCell<F> {
        AllocatedUnlimitedCell(self.alloc(&ETableCellType::Unlimited))
    }
}
