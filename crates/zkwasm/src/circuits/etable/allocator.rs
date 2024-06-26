use super::AllocatedU32StateCell;
use super::EVENT_TABLE_ENTRY_ROWS;
use crate::circuits::bit_table::BitTableOp;
use crate::circuits::cell::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::rtable::RangeTableConfig;
use crate::circuits::traits::ConfigureLookupTable;
use crate::circuits::utils::bit::BitColumn;
use crate::circuits::utils::common_range::CommonRangeColumn;
use crate::circuits::utils::u16::U16Column;
use crate::circuits::utils::u8::U8Column;
use crate::circuits::Context;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::memory_table::encode_memory_table_entry;
use specs::mtable::LocationType;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub(super) trait EventTableCellExpression<F: FieldExt> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
}

impl<F: FieldExt> EventTableCellExpression<F> for AllocatedCell<F> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot + EVENT_TABLE_ENTRY_ROWS)
    }

    fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot - EVENT_TABLE_ENTRY_ROWS)
    }
}

macro_rules! impl_cell {
    ($x: ident) => {
        impl<F: FieldExt> EventTableCellExpression<F> for $x<F> {
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
impl_cell!(AllocatedU8Cell);
impl_cell!(AllocatedU16Cell);
impl_cell!(AllocatedCommonRangeCell);
impl_cell!(AllocatedUnlimitedCell);

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedMemoryTableLookupReadCell<F: FieldExt> {
    pub(crate) encode_cell: AllocatedUnlimitedCell<F>,
    pub(crate) start_eid_cell: AllocatedUnlimitedCell<F>,
    pub(crate) end_eid_cell: AllocatedUnlimitedCell<F>,
    pub(crate) start_eid_diff_cell: AllocatedU32StateCell<F>,
    pub(crate) end_eid_diff_cell: AllocatedU32StateCell<F>,
    pub(crate) value_cell: AllocatedUnlimitedCell<F>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedMemoryTableLookupWriteCell<F: FieldExt> {
    pub(crate) encode_cell: AllocatedUnlimitedCell<F>,
    pub(crate) start_eid_cell: AllocatedUnlimitedCell<F>,
    pub(crate) end_eid_cell: AllocatedUnlimitedCell<F>,
    pub(crate) value_cell: AllocatedUnlimitedCell<F>,
}

impl<F: FieldExt> AllocatedMemoryTableLookupReadCell<F> {
    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        start_eid: u32,
        eid: u32,
        end_eid: u32,
        offset: u32,
        l_type: LocationType,
        is_i32: bool,
        value: u64,
    ) -> Result<(), Error> {
        self.encode_cell.assign_bn(
            ctx,
            &encode_memory_table_entry(
                (offset as u64).into(),
                (l_type as u64).into(),
                (is_i32 as u64).into(),
            ),
        )?;
        self.start_eid_cell.assign_u32(ctx, start_eid)?;
        cfg_if::cfg_if! {
            if #[cfg(feature="continuation")] {
                self.start_eid_diff_cell.assign(ctx, eid - start_eid - 1)?;
                self.end_eid_diff_cell.assign(ctx, end_eid - eid)?;
            } else {
                self.start_eid_diff_cell.assign_u32(ctx, eid - start_eid - 1)?;
                self.end_eid_diff_cell.assign_u32(ctx, end_eid - eid)?;
            }
        }
        self.end_eid_cell.assign_u32(ctx, end_eid)?;
        self.value_cell.assign(ctx, value.into())?;

        Ok(())
    }
}

impl<F: FieldExt> AllocatedMemoryTableLookupWriteCell<F> {
    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        eid: u32,
        end_eid: u32,
        offset: u32,
        l_type: LocationType,
        is_i32: bool,
        value: u64,
    ) -> Result<(), Error> {
        self.encode_cell.assign_bn(
            ctx,
            &encode_memory_table_entry(
                (offset as u64).into(),
                (l_type as u64).into(),
                (is_i32 as u64).into(),
            ),
        )?;
        self.start_eid_cell.assign_u32(ctx, eid)?;
        self.end_eid_cell.assign_u32(ctx, end_eid)?;
        self.value_cell.assign(ctx, value.into())?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EventTableCellType {
    Bit = 1,
    U8,
    U16,
    CommonRange,
    Unlimited,
    MTableLookup,
}

const BIT_COLUMNS: usize = 12;
const U8_COLUMNS: usize = 1;
const U32_CELLS: usize = 2;
const U32_PERMUTATION_CELLS: usize = if cfg!(feature = "continuation") {
    10
} else {
    0
};
const U64_CELLS: usize = 5;
const U16_COLUMNS: usize =
    U64_CELLS + ((U32_CELLS + U32_PERMUTATION_CELLS).next_multiple_of(2) / 2);
const COMMON_RANGE_COLUMNS: usize = if cfg!(feature = "continuation") { 4 } else { 6 };
const UNLIMITED_COLUMNS: usize = if cfg!(feature = "continuation") {
    10
} else {
    8
};
const MEMORY_TABLE_LOOKUP_COLUMNS: usize = 2;

#[derive(Clone, Copy)]
pub(crate) struct AllocatedBitTableLookupCells<F: FieldExt> {
    pub(crate) op: AllocatedUnlimitedCell<F>,
    pub(crate) left: AllocatedUnlimitedCell<F>,
    pub(crate) right: AllocatedUnlimitedCell<F>,
    pub(crate) result: AllocatedUnlimitedCell<F>,
}

impl<F: FieldExt> AllocatedBitTableLookupCells<F> {
    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        op: BitTableOp,
        left: u64,
        right: u64,
        result: u64,
    ) -> Result<(), Error> {
        self.op.assign(ctx, F::from(op.index() as u64))?;
        self.left.assign(ctx, F::from(left))?;
        self.right.assign(ctx, F::from(right))?;
        self.result.assign(ctx, F::from(result))?;

        Ok(())
    }
}

pub(crate) struct AllocatorFreeCellsProfiler {
    free_cells: BTreeMap<EventTableCellType, (usize, u32)>,
    free_u32_cells: usize,
    free_u32_permutation_cells: usize,
    free_u64_cells: usize,
}

impl AllocatorFreeCellsProfiler {
    pub(crate) fn new<F: FieldExt>(allocator: &EventTableCellAllocator<F>) -> Self {
        Self {
            free_cells: allocator.free_cells.clone(),
            free_u32_cells: allocator.free_u32_cells.len(),
            free_u32_permutation_cells: allocator.free_u32_permutation_cells.len(),
            free_u64_cells: allocator.free_u64_cells.len(),
        }
    }

    pub(crate) fn update<F: FieldExt>(&mut self, allocator: &EventTableCellAllocator<F>) {
        for (t, (i, j)) in allocator.free_cells.iter() {
            let v = self.free_cells.get_mut(t).unwrap();

            match i.cmp(&v.0) {
                Ordering::Greater => {
                    v.0 = *i;
                    v.1 = *j;
                }
                Ordering::Equal => v.1 = u32::max(v.1, *j),
                Ordering::Less => (),
            }
        }

        self.free_u32_cells = usize::min(self.free_u32_cells, allocator.free_u32_cells.len());
        self.free_u32_permutation_cells = usize::min(
            self.free_u32_permutation_cells,
            allocator.free_u32_permutation_cells.len(),
        );
        self.free_u64_cells = usize::min(self.free_u64_cells, allocator.free_u64_cells.len());
    }

    pub(crate) fn assert_no_free_cells<F: FieldExt>(&self, allocator: &EventTableCellAllocator<F>) {
        for (t, (i, j)) in &self.free_cells {
            let cols = allocator.all_cols.get(t).unwrap();

            assert!(
                *i == cols.len() || (*i == cols.len() - 1 && *j > 0),
                "unused {:?} col should be removed: {}.",
                t,
                cols.len() - *i - (*j != 0) as usize
            );
        }

        assert!(
            self.free_u32_cells == 0,
            "unused u32 cells should be removed: {:?}.",
            self.free_u32_cells
        );
        assert!(
            self.free_u32_permutation_cells == 0,
            "unused u32 permutation cells should be removed: {:?}.",
            self.free_u32_permutation_cells
        );
        assert!(
            self.free_u64_cells == 0,
            "unused u64 cells should be removed: {:?}.",
            self.free_u64_cells
        );
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EventTableCellAllocator<F: FieldExt> {
    pub(crate) free_cells: BTreeMap<EventTableCellType, (usize, u32)>,
    all_cols: BTreeMap<EventTableCellType, Vec<Vec<Column<Advice>>>>,
    free_u32_cells: Vec<AllocatedU32Cell<F>>,
    free_u32_permutation_cells: Vec<AllocatedU32PermutationCell<F>>,
    free_u64_cells: Vec<AllocatedU64Cell<F>>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableCellAllocator<F> {
    pub fn enable_equality(
        &mut self,
        meta: &mut ConstraintSystem<F>,
        t: &EventTableCellType,
        count: usize,
    ) {
        for c in self.all_cols.get(t).unwrap().iter().take(count) {
            for c in c {
                meta.enable_equality(*c);
            }
        }
    }

    pub(super) fn prepare_alloc_u32_cell(&mut self) -> AllocatedU32Cell<F> {
        let u16_cells_le = [0; 2].map(|_| self.alloc_u16_cell());

        AllocatedU32Cell { u16_cells_le }
    }

    pub(super) fn prepare_alloc_u32_permutation_cell(
        &mut self,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> AllocatedU32PermutationCell<F> {
        let u16_cells_le = [0; 2].map(|_| self.alloc_u16_cell());
        let u32_cell = self.alloc_unlimited_cell();
        meta.create_gate("c9. u32 decompose", |meta| {
            let init = u32_cell.curr_expr(meta);
            vec![
                (0..2)
                    .map(|x| u16_cells_le[x].curr_expr(meta) * constant_from!(1u64 << (16 * x)))
                    .fold(init, |acc, x| acc - x)
                    * enable(meta),
            ]
        });
        meta.enable_equality(u32_cell.cell.col);

        AllocatedU32PermutationCell {
            u16_cells_le,
            u32_cell,
        }
    }

    pub(super) fn prepare_alloc_u64_cell(
        &mut self,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> AllocatedU64Cell<F> {
        let u16_cells_le = [0; 4].map(|_| self.alloc_u16_cell());
        let u64_cell = self.alloc_unlimited_cell();
        meta.create_gate("c9. u64 decompose", |meta| {
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
        mtable: &impl ConfigureLookupTable<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut allocator = Self::_new(
            meta,
            sel,
            (l_0, l_active, l_active_last),
            rtable,
            mtable,
            cols,
        );
        for _ in 0..U32_CELLS {
            let cell = allocator.prepare_alloc_u32_cell();
            allocator.free_u32_cells.push(cell);
        }
        #[allow(clippy::reversed_empty_ranges)]
        for _ in 0..U32_PERMUTATION_CELLS {
            let cell =
                allocator.prepare_alloc_u32_permutation_cell(meta, |meta| fixed_curr!(meta, sel));
            allocator.free_u32_permutation_cells.push(cell);
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
        mtable: &impl ConfigureLookupTable<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut all_cols = BTreeMap::new();
        all_cols.insert(
            EventTableCellType::Bit,
            [0; BIT_COLUMNS]
                .map(|_| vec![BitColumn::configure(meta, cols, |meta| fixed_curr!(meta, sel)).col])
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::U8,
            [0; U8_COLUMNS]
                .map(|_| vec![U8Column::configure(meta, (l_0, l_active, l_active_last)).col])
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::U16,
            [0; U16_COLUMNS]
                .map(|_| vec![U16Column::configure(meta, (l_0, l_active, l_active_last)).col])
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::CommonRange,
            [0; COMMON_RANGE_COLUMNS]
                .map(|_| {
                    vec![
                        CommonRangeColumn::configure(meta, cols, rtable, |_| constant_from!(1)).col,
                    ]
                })
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::Unlimited,
            [0; UNLIMITED_COLUMNS]
                .map(|_| vec![cols.next().unwrap()])
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::MTableLookup,
            [0; MEMORY_TABLE_LOOKUP_COLUMNS]
                .map(|_| {
                    let start_eid_col = cols.next().unwrap();
                    let end_eid_col = cols.next().unwrap();
                    let encode_col = cols.next().unwrap();
                    let value_col = cols.next().unwrap();
                    mtable.configure_in_table(meta, "c8e. mtable_lookup in mtable", |meta| {
                        vec![
                            curr!(meta, start_eid_col),
                            curr!(meta, end_eid_col),
                            curr!(meta, encode_col),
                            curr!(meta, value_col),
                        ]
                    });
                    vec![start_eid_col, end_eid_col, encode_col, value_col]
                })
                .into_iter()
                .collect(),
        );

        Self {
            all_cols,
            free_cells: BTreeMap::from_iter(
                vec![
                    (EventTableCellType::Bit, (0, 0)),
                    (EventTableCellType::U8, (0, 0)),
                    (EventTableCellType::U16, (0, 0)),
                    (EventTableCellType::CommonRange, (0, 0)),
                    (EventTableCellType::Unlimited, (0, 0)),
                    (EventTableCellType::MTableLookup, (0, 0)),
                ]
                .into_iter(),
            ),
            free_u32_cells: vec![],
            free_u32_permutation_cells: vec![],
            free_u64_cells: vec![],
            _mark: PhantomData,
        }
    }

    fn alloc(&mut self, t: &EventTableCellType) -> AllocatedCell<F> {
        let v = self.free_cells.get_mut(t).unwrap();

        let res = AllocatedCell {
            col: self.all_cols.get(t).unwrap()[v.0][0],
            rot: v.1 as i32,
            _mark: PhantomData,
        };

        v.1 += 1;
        if v.1 == EVENT_TABLE_ENTRY_ROWS as u32 {
            v.0 += 1;
            v.1 = 0;
        }

        res
    }

    fn alloc_group(&mut self, t: &EventTableCellType) -> Vec<AllocatedCell<F>> {
        let v = self.free_cells.get_mut(t).unwrap();

        let res = self.all_cols.get(t).unwrap()[v.0]
            .iter()
            .map(|col| AllocatedCell {
                col: *col,
                rot: v.1 as i32,
                _mark: PhantomData,
            })
            .collect();

        v.1 += 1;
        if v.1 == EVENT_TABLE_ENTRY_ROWS as u32 {
            v.0 += 1;
            v.1 = 0;
        }

        res
    }

    pub(crate) fn alloc_bit_cell(&mut self) -> AllocatedBitCell<F> {
        AllocatedBitCell {
            cell: self.alloc(&EventTableCellType::Bit),
        }
    }

    pub(crate) fn alloc_common_range_cell(&mut self) -> AllocatedCommonRangeCell<F> {
        AllocatedCommonRangeCell {
            cell: self.alloc(&EventTableCellType::CommonRange),
        }
    }

    pub(crate) fn alloc_u32_state_cell(&mut self) -> AllocatedU32StateCell<F> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "continuation")] {
                self.alloc_u32_permutation_cell()
            } else {
                self.alloc_common_range_cell()
            }
        }
    }

    pub(crate) fn alloc_u8_cell(&mut self) -> AllocatedU8Cell<F> {
        AllocatedU8Cell {
            cell: self.alloc(&EventTableCellType::U8),
        }
    }

    pub(crate) fn alloc_u16_cell(&mut self) -> AllocatedU16Cell<F> {
        AllocatedU16Cell {
            cell: self.alloc(&EventTableCellType::U16),
        }
    }

    pub(crate) fn alloc_unlimited_cell(&mut self) -> AllocatedUnlimitedCell<F> {
        AllocatedUnlimitedCell {
            cell: self.alloc(&EventTableCellType::Unlimited),
        }
    }

    pub(crate) fn alloc_memory_table_lookup_read_cell(
        &mut self,
        name: &'static str,
        constraint_builder: &mut ConstraintBuilder<F>,
        eid: AllocatedU32StateCell<F>,
        location_type: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        offset: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedMemoryTableLookupReadCell<F> {
        let cells: Vec<_> = self
            .alloc_group(&EventTableCellType::MTableLookup)
            .into_iter()
            .map(|x| AllocatedUnlimitedCell { cell: x })
            .collect();

        let cell = AllocatedMemoryTableLookupReadCell {
            start_eid_cell: cells[0],
            end_eid_cell: cells[1],
            encode_cell: cells[2],
            value_cell: cells[3],
            start_eid_diff_cell: self.alloc_u32_state_cell(),
            end_eid_diff_cell: self.alloc_u32_state_cell(),
        };

        constraint_builder.constraints.push((
            name,
            Box::new(move |meta| {
                let enable = enable(meta);
                vec![
                    (eid.expr(meta)
                        - cell.start_eid_cell.expr(meta)
                        - cell.start_eid_diff_cell.expr(meta)
                        - constant_from!(1))
                        * enable.clone(),
                    (eid.expr(meta) + cell.end_eid_diff_cell.expr(meta)
                        - cell.end_eid_cell.expr(meta))
                        * enable.clone(),
                    (encode_memory_table_entry(offset(meta), location_type(meta), is_i32(meta))
                        - cell.encode_cell.expr(meta))
                        * enable.clone(),
                    (cell.value_cell.expr(meta) - value(meta)) * enable,
                ]
            }),
        ));

        cell
    }

    pub(crate) fn alloc_memory_table_lookup_write_cell(
        &mut self,
        name: &'static str,
        constraint_builder: &mut ConstraintBuilder<F>,
        eid: AllocatedU32StateCell<F>,
        location_type: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        offset: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedMemoryTableLookupWriteCell<F> {
        let cells: Vec<_> = self
            .alloc_group(&EventTableCellType::MTableLookup)
            .into_iter()
            .map(|x| AllocatedUnlimitedCell { cell: x })
            .collect();

        let cell = AllocatedMemoryTableLookupWriteCell {
            start_eid_cell: cells[0],
            end_eid_cell: cells[1],
            encode_cell: cells[2],
            value_cell: cells[3],
        };

        constraint_builder.constraints.push((
            name,
            Box::new(move |meta| {
                let enable = enable(meta);
                vec![
                    (encode_memory_table_entry(offset(meta), location_type(meta), is_i32(meta))
                        - cell.encode_cell.expr(meta))
                        * enable.clone(),
                    (cell.start_eid_cell.expr(meta) - eid.expr(meta)) * enable.clone(),
                    (cell.value_cell.expr(meta) - value(meta)) * enable,
                ]
            }),
        ));

        cell
    }

    pub(crate) fn alloc_memory_table_lookup_read_cell_with_value(
        &mut self,
        name: &'static str,
        constraint_builder: &mut ConstraintBuilder<F>,
        eid: AllocatedU32StateCell<F>,
        location_type: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        offset: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedMemoryTableLookupReadCell<F> {
        let cells: Vec<_> = self
            .alloc_group(&EventTableCellType::MTableLookup)
            .into_iter()
            .map(|x| AllocatedUnlimitedCell { cell: x })
            .collect();

        let cell = AllocatedMemoryTableLookupReadCell {
            start_eid_cell: cells[0],
            end_eid_cell: cells[1],
            encode_cell: cells[2],
            value_cell: cells[3],
            start_eid_diff_cell: self.alloc_u32_state_cell(),
            end_eid_diff_cell: self.alloc_u32_state_cell(),
        };

        constraint_builder.constraints.push((
            name,
            Box::new(move |meta| {
                let enable = enable(meta);
                vec![
                    (eid.expr(meta)
                        - cell.start_eid_cell.expr(meta)
                        - cell.start_eid_diff_cell.expr(meta)
                        - constant_from!(1))
                        * enable.clone(),
                    (eid.expr(meta) + cell.end_eid_diff_cell.expr(meta)
                        - cell.end_eid_cell.expr(meta))
                        * enable.clone(),
                    (encode_memory_table_entry(offset(meta), location_type(meta), is_i32(meta))
                        - cell.encode_cell.expr(meta))
                        * enable,
                ]
            }),
        ));

        cell
    }

    pub(crate) fn alloc_memory_table_lookup_write_cell_with_value(
        &mut self,
        name: &'static str,
        constraint_builder: &mut ConstraintBuilder<F>,
        eid: AllocatedU32StateCell<F>,
        location_type: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        offset: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedMemoryTableLookupWriteCell<F> {
        let cells: Vec<_> = self
            .alloc_group(&EventTableCellType::MTableLookup)
            .into_iter()
            .map(|x| AllocatedUnlimitedCell { cell: x })
            .collect();

        let cell = AllocatedMemoryTableLookupWriteCell {
            start_eid_cell: cells[0],
            end_eid_cell: cells[1],
            encode_cell: cells[2],
            value_cell: cells[3],
        };

        constraint_builder.constraints.push((
            name,
            Box::new(move |meta| {
                let enable = enable(meta);
                vec![
                    (encode_memory_table_entry(offset(meta), location_type(meta), is_i32(meta))
                        - cell.encode_cell.expr(meta))
                        * enable.clone(),
                    (cell.start_eid_cell.expr(meta) - eid.expr(meta)) * enable,
                ]
            }),
        ));

        cell
    }

    pub(crate) fn alloc_u32_cell(&mut self) -> AllocatedU32Cell<F> {
        self.free_u32_cells.pop().expect("no more free u32 cells")
    }

    #[allow(dead_code)]
    pub(crate) fn alloc_u32_permutation_cell(&mut self) -> AllocatedU32PermutationCell<F> {
        self.free_u32_permutation_cells
            .pop()
            .expect("no more free u32 permutation cells")
    }

    pub(crate) fn alloc_u64_cell(&mut self) -> AllocatedU64Cell<F> {
        self.free_u64_cells.pop().expect("no more free u64 cells")
    }

    pub(crate) fn alloc_u64_with_flag_bit_cell_dyn(
        &mut self,
        constraint_builder: &mut ConstraintBuilder<F>,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedU64CellWithFlagBitDyn<F> {
        let value = self.free_u64_cells.pop().expect("no more free u64 cells");
        let flag_bit_cell = self.alloc_bit_cell();
        let flag_u16_rem_cell = self.alloc_common_range_cell();
        let flag_u16_rem_diff_cell = self.alloc_common_range_cell();

        constraint_builder.push(
            "flag bit dyn",
            Box::new(move |meta| {
                let flag_u16 = value.u16_cells_le[3].expr(meta)
                    + is_i32(meta)
                        * (value.u16_cells_le[1].expr(meta) - value.u16_cells_le[3].expr(meta));
                vec![
                    (flag_bit_cell.expr(meta) * constant_from!(1 << 15)
                        + flag_u16_rem_cell.expr(meta)
                        - flag_u16),
                    (flag_u16_rem_cell.expr(meta) + flag_u16_rem_diff_cell.expr(meta)
                        - constant_from!((1 << 15) - 1)),
                ]
            }),
        );

        AllocatedU64CellWithFlagBitDyn {
            u16_cells_le: value.u16_cells_le,
            u64_cell: value.u64_cell,
            flag_bit_cell,
            flag_u16_rem_cell,
            flag_u16_rem_diff_cell,
        }
    }

    pub(crate) fn alloc_u64_with_flag_bit_cell_dyn_sign(
        &mut self,
        constraint_builder: &mut ConstraintBuilder<F>,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_sign: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedU64CellWithFlagBitDynSign<F> {
        let value = self.free_u64_cells.pop().expect("no more free u64 cells");
        let flag_bit_cell = self.alloc_bit_cell();
        let flag_u16_rem_cell = self.alloc_common_range_cell();
        let flag_u16_rem_diff_cell = self.alloc_common_range_cell();

        constraint_builder.push(
            "flag bit dyn sign",
            Box::new(move |meta| {
                let flag_u16 = value.u16_cells_le[3].expr(meta)
                    + is_i32(meta)
                        * (value.u16_cells_le[1].expr(meta) - value.u16_cells_le[3].expr(meta));
                let is_sign = is_sign(meta);
                vec![
                    is_sign.clone()
                        * (flag_bit_cell.expr(meta) * constant_from!(1 << 15)
                            + flag_u16_rem_cell.expr(meta)
                            - flag_u16),
                    is_sign.clone()
                        * (flag_u16_rem_cell.expr(meta) + flag_u16_rem_diff_cell.expr(meta)
                            - constant_from!((1 << 15) - 1)),
                    (is_sign - constant_from!(1)) * flag_bit_cell.expr(meta),
                ]
            }),
        );

        AllocatedU64CellWithFlagBitDynSign {
            u16_cells_le: value.u16_cells_le,
            u64_cell: value.u64_cell,
            flag_bit_cell,
            flag_u16_rem_cell,
            flag_u16_rem_diff_cell,
        }
    }

    pub(crate) fn alloc_bit_table_lookup_cells(&mut self) -> AllocatedBitTableLookupCells<F> {
        AllocatedBitTableLookupCells {
            op: self.alloc_unlimited_cell(),
            left: self.alloc_unlimited_cell(),
            right: self.alloc_unlimited_cell(),
            result: self.alloc_unlimited_cell(),
        }
    }
}
