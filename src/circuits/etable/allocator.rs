use super::EVENT_TABLE_ENTRY_ROWS;
use crate::circuits::bit_table::BitTableConfig;
use crate::circuits::cell::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::jtable::JumpTableConfig;
use crate::circuits::rtable::RangeTableConfig;
use crate::circuits::traits::ConfigureLookupTable;
use crate::circuits::utils::bit::BitColumn;
use crate::circuits::utils::common_range::CommonRangeColumn;
use crate::circuits::utils::u16::U16Column;
use crate::circuits::utils::u8::U8Column;
use crate::circuits::Context;
use crate::circuits::Lookup;
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
use std::collections::BTreeMap;
use std::marker::PhantomData;

pub(super) trait EventTableCellExpression<F: FieldExt> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
}

impl<F: FieldExt> EventTableCellExpression<F> for AllocatedCell<F> {
    fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot + EVENT_TABLE_ENTRY_ROWS as i32)
    }

    fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot - EVENT_TABLE_ENTRY_ROWS as i32)
    }
}

macro_rules! impl_cell {
    ($x: ident) => {
        impl<F: FieldExt> EventTableCellExpression<F> for $x<F> {
            fn next_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
                self.0.next_expr(meta)
            }

            fn prev_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
                self.0.prev_expr(meta)
            }
        }
    };
}

impl_cell!(AllocatedBitCell);
impl_cell!(AllocatedCommonRangeCell);
impl_cell!(AllocatedU8Cell);
impl_cell!(AllocatedU16Cell);
impl_cell!(AllocatedUnlimitedCell);
impl_cell!(AllocatedJumpTableLookupCell);
impl_cell!(AllocatedBitTableLookupCell);

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedJumpTableLookupCell<F: FieldExt>(pub(crate) AllocatedCell<F>);

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedBitTableLookupCell<F: FieldExt>(pub(crate) AllocatedCell<F>);

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedMemoryTableLookupReadCell<F: FieldExt> {
    pub(crate) encode_cell: AllocatedUnlimitedCell<F>,
    pub(crate) start_eid_cell: AllocatedCommonRangeCell<F>,
    pub(crate) end_eid_cell: AllocatedCommonRangeCell<F>,
    pub(crate) start_eid_diff_cell: AllocatedCommonRangeCell<F>,
    pub(crate) end_eid_diff_cell: AllocatedCommonRangeCell<F>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AllocatedMemoryTableLookupWriteCell<F: FieldExt> {
    pub(crate) encode_cell: AllocatedUnlimitedCell<F>,
    pub(crate) end_eid_cell: AllocatedCommonRangeCell<F>,
    pub(crate) end_eid_diff_cell: AllocatedCommonRangeCell<F>,
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
                (start_eid as u64).into(),
                (end_eid as u64).into(),
                (offset as u64).into(),
                (l_type as u64).into(),
                (is_i32 as u64).into(),
                value.into(),
            ),
        )?;
        self.start_eid_cell.assign_u32(ctx, start_eid)?;
        self.start_eid_diff_cell
            .assign_u32(ctx, eid - start_eid - 1)?;
        self.end_eid_cell.assign_u32(ctx, end_eid)?;
        self.end_eid_diff_cell.assign_u32(ctx, end_eid - eid)?;

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
                (eid as u64).into(),
                (end_eid as u64).into(),
                (offset as u64).into(),
                (l_type as u64).into(),
                (is_i32 as u64).into(),
                value.into(),
            ),
        )?;
        self.end_eid_cell.assign_u32(ctx, end_eid)?;
        self.end_eid_diff_cell.assign_u32(ctx, end_eid - eid - 1)?;

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
    JTableLookup,
    BitTableLookup,
}

const BIT_COLUMNS: usize = 12;
const U8_COLUMNS: usize = 1;
const U16_COLUMNS: usize = 10;
const COMMON_RANGE_COLUMNS: usize = 9;
const UNLIMITED_COLUMNS: usize = 6;
const MEMORY_TABLE_LOOKUP_COLUMNS: usize = 2;
const JUMP_TABLE_LOOKUP_COLUMNS: usize = 1;
const BIT_TABLE_LOOKUP_COLUMNS: usize = 1;
const U64_CELLS: usize = 10;

#[derive(Debug, Clone)]
pub(crate) struct EventTableCellAllocator<F: FieldExt> {
    all_cols: BTreeMap<EventTableCellType, Vec<Column<Advice>>>,
    free_cells: BTreeMap<EventTableCellType, (usize, u32)>,
    free_u64_cells: Vec<AllocatedU64Cell<F>>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> EventTableCellAllocator<F> {
    pub fn enable_equality(&mut self, meta: &mut ConstraintSystem<F>, t: &EventTableCellType) {
        for c in self.all_cols.get(t).unwrap() {
            meta.enable_equality(*c);
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
                    .into_iter()
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
        rtable: &RangeTableConfig<F>,
        mtable: &impl ConfigureLookupTable<F>,
        jtable: &JumpTableConfig<F>,
        bit_table: &BitTableConfig<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut allocator = Self::_new(meta, sel, rtable, mtable, jtable, bit_table, cols);
        for _ in 0..U64_CELLS {
            let cell = allocator.prepare_alloc_u64_cell(meta, |meta| fixed_curr!(meta, sel));
            allocator.free_u64_cells.push(cell);
        }
        allocator
    }

    fn _new(
        meta: &mut ConstraintSystem<F>,
        sel: Column<Fixed>,
        rtable: &RangeTableConfig<F>,
        mtable: &impl ConfigureLookupTable<F>,
        jtable: &JumpTableConfig<F>,
        bit_table: &BitTableConfig<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let mut all_cols = BTreeMap::new();
        all_cols.insert(
            EventTableCellType::Bit,
            [0; BIT_COLUMNS]
                .map(|_| BitColumn::configure(meta, cols, |meta| fixed_curr!(meta, sel)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::U8,
            [0; U8_COLUMNS]
                .map(|_| U8Column::configure(meta, cols, rtable, |_| constant_from!(1)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::U16,
            [0; U16_COLUMNS]
                .map(|_| U16Column::configure(meta, cols, rtable, |_| constant_from!(1)).col)
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::CommonRange,
            [0; COMMON_RANGE_COLUMNS]
                .map(|_| {
                    CommonRangeColumn::configure(meta, cols, rtable, |_| constant_from!(1)).col
                })
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::Unlimited,
            [0; UNLIMITED_COLUMNS]
                .map(|_| cols.next().unwrap())
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::JTableLookup,
            [0; JUMP_TABLE_LOOKUP_COLUMNS]
                .map(|_| {
                    let col = cols.next().unwrap();
                    jtable.configure_in_table(meta, "c8c. jtable_lookup in jtable", |meta| {
                        curr!(meta, col)
                    });
                    col
                })
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::MTableLookup,
            [0; MEMORY_TABLE_LOOKUP_COLUMNS]
                .map(|_| {
                    let col = cols.next().unwrap();
                    mtable.configure_in_table(meta, "c8e. mtable_lookup in mtable", |meta| {
                        vec![curr!(meta, col)]
                    });
                    col
                })
                .into_iter()
                .collect(),
        );
        all_cols.insert(
            EventTableCellType::BitTableLookup,
            [0; BIT_TABLE_LOOKUP_COLUMNS]
                .map(|_| {
                    let col = cols.next().unwrap();
                    bit_table.configure_in_table(
                        meta,
                        "c8f: bit_table_lookup in bit_table",
                        |meta| curr!(meta, col),
                    );
                    col
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
                    (EventTableCellType::JTableLookup, (0, 0)),
                    (EventTableCellType::BitTableLookup, (0, 0)),
                ]
                .into_iter(),
            ),
            free_u64_cells: vec![],
            _mark: PhantomData,
        }
    }

    fn alloc(&mut self, t: &EventTableCellType) -> AllocatedCell<F> {
        let v = self.free_cells.get_mut(t).unwrap();

        let res = AllocatedCell {
            col: self.all_cols.get(t).unwrap()[v.0],
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

    pub(crate) fn alloc_bit_cell(&mut self) -> AllocatedBitCell<F> {
        AllocatedBitCell(self.alloc(&EventTableCellType::Bit))
    }

    pub(crate) fn alloc_common_range_cell(&mut self) -> AllocatedCommonRangeCell<F> {
        AllocatedCommonRangeCell(self.alloc(&EventTableCellType::CommonRange))
    }

    pub(crate) fn alloc_u8_cell(&mut self) -> AllocatedU8Cell<F> {
        AllocatedU8Cell(self.alloc(&EventTableCellType::U8))
    }

    pub(crate) fn alloc_u16_cell(&mut self) -> AllocatedU16Cell<F> {
        AllocatedU16Cell(self.alloc(&EventTableCellType::U16))
    }

    pub(crate) fn alloc_unlimited_cell(&mut self) -> AllocatedUnlimitedCell<F> {
        AllocatedUnlimitedCell(self.alloc(&EventTableCellType::Unlimited))
    }

    pub(crate) fn alloc_jump_table_lookup_cell(&mut self) -> AllocatedJumpTableLookupCell<F> {
        AllocatedJumpTableLookupCell(self.alloc(&EventTableCellType::JTableLookup))
    }

    pub(crate) fn alloc_bit_table_lookup_cell(&mut self) -> AllocatedBitTableLookupCell<F> {
        AllocatedBitTableLookupCell(self.alloc(&EventTableCellType::BitTableLookup))
    }

    pub(crate) fn alloc_memory_table_lookup_read_cell(
        &mut self,
        name: &'static str,
        constraint_builder: &mut ConstraintBuilder<F>,
        eid: AllocatedCommonRangeCell<F>,
        location_type: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        offset: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedMemoryTableLookupReadCell<F> {
        let cell = AllocatedMemoryTableLookupReadCell {
            encode_cell: AllocatedUnlimitedCell(self.alloc(&EventTableCellType::MTableLookup)),
            start_eid_cell: self.alloc_common_range_cell(),
            end_eid_cell: self.alloc_common_range_cell(),
            start_eid_diff_cell: self.alloc_common_range_cell(),
            end_eid_diff_cell: self.alloc_common_range_cell(),
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
                    (encode_memory_table_entry(
                        cell.start_eid_cell.expr(meta),
                        cell.end_eid_cell.expr(meta),
                        offset(meta),
                        location_type(meta),
                        is_i32(meta),
                        value(meta),
                    ) - cell.encode_cell.expr(meta))
                        * enable,
                ]
            }),
        ));

        cell
    }

    pub(crate) fn alloc_memory_table_lookup_write_cell(
        &mut self,
        name: &'static str,
        constraint_builder: &mut ConstraintBuilder<F>,
        eid: AllocatedCommonRangeCell<F>,
        location_type: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        offset: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        is_i32: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F> + 'static,
    ) -> AllocatedMemoryTableLookupWriteCell<F> {
        let cell = AllocatedMemoryTableLookupWriteCell {
            encode_cell: AllocatedUnlimitedCell(self.alloc(&EventTableCellType::MTableLookup)),
            end_eid_cell: self.alloc_common_range_cell(),
            end_eid_diff_cell: self.alloc_common_range_cell(),
        };

        constraint_builder.constraints.push((
            name,
            Box::new(move |meta| {
                let enable = enable(meta);
                vec![
                    (eid.expr(meta) + cell.end_eid_diff_cell.expr(meta) + constant_from!(1)
                        - cell.end_eid_cell.expr(meta))
                        * enable.clone(),
                    (encode_memory_table_entry(
                        eid.expr(meta),
                        cell.end_eid_cell.expr(meta),
                        offset(meta),
                        location_type(meta),
                        is_i32(meta),
                        value(meta),
                    ) - cell.encode_cell.expr(meta))
                        * enable,
                ]
            }),
        ));

        cell
    }

    pub(crate) fn alloc_u64_cell(&mut self) -> AllocatedU64Cell<F> {
        self.free_u64_cells.pop().expect("no more free u64 cells")
    }

    pub(crate) fn alloc_u64_with_flag_bit_cell<const POS: usize>(
        &mut self,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> AllocatedU64CellWithFlagBit<F, POS> {
        let value = self.free_u64_cells.pop().expect("no more free u64 cells");
        let flag_bit_cell = self.alloc_bit_cell();
        let flag_u16_rem_cell = self.alloc_common_range_cell();
        let flag_u16_rem_diff_cell = self.alloc_common_range_cell();

        constraint_builder.push(
            "flag bit",
            Box::new(move |meta| {
                let flag_u16 = value.u16_cells_le[POS].expr(meta);
                vec![
                    (flag_bit_cell.expr(meta) * constant_from!(1 << 15)
                        + flag_u16_rem_cell.expr(meta)
                        - flag_u16),
                    (flag_u16_rem_cell.expr(meta) + flag_u16_rem_diff_cell.expr(meta)
                        - constant_from!((1 << 15) - 1)),
                ]
            }),
        );

        AllocatedU64CellWithFlagBit {
            u16_cells_le: value.u16_cells_le,
            u64_cell: value.u64_cell,
            flag_bit_cell,
            flag_u16_rem_cell,
            flag_u16_rem_diff_cell,
        }
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
}
