use self::configure::MemoryTableConstriants;
use super::{
    config::max_mtable_rows,
    imtable::InitMemoryTableConfig,
    rtable::RangeTableConfig,
    utils::{row_diff::RowDiffConfig, Context},
    CircuitConfigure,
};
use crate::circuits::{mtable_compact::configure::STEP_SIZE, IMTABLE_COLOMNS};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Cell,
    plonk::{Advice, Column, ConstraintSystem, Error, Fixed},
};
use specs::mtable::{AccessType, InitType, LocationType, MTable, MemoryTableEntry, VarType};

fn mtable_rows() -> usize {
    max_mtable_rows() as usize / STEP_SIZE as usize * STEP_SIZE as usize
}

pub mod configure;
pub(crate) mod encode;
pub mod expression;

enum RotationOfIndexColumn {
    LTYPE = 0,
    MMID,
    OFFSET,
    EID,
    EMID,
    MAX,
}

pub enum RotationOfAuxColumn {
    ConstantOne = 0,
    SameLtype,
    SameMmid,
    SameOffset,
    SameEid,
    Atype,
    RestMops,
    RangeInLazyInitDiff,
}

pub enum RotationOfBitColumn {
    Enable = 0,
    Is64Bit,
    IsStack,
    IsMutable,
    IsLazyInit,
    // To support multiple imtable columns,
    // the seletors is a bit filter for an imtable lookup.
    IMTableSelectorStart,
}

#[derive(Clone)]
pub struct MemoryTableConfig<F: FieldExt> {
    pub(crate) sel: Column<Fixed>,
    pub(crate) following_block_sel: Column<Fixed>,
    pub(crate) block_first_line_sel: Column<Fixed>,

    // See enum RotationOfBitColumn
    pub(crate) bit: Column<Advice>,

    // See enum RotationOfIndexColumn
    pub(crate) index: RowDiffConfig<F>,

    // See enum RotationOfBitColumn
    pub(crate) aux: Column<Advice>,

    // Rotation:
    // 0..8 bytes
    pub(crate) bytes: Column<Advice>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
        configure: &CircuitConfigure,
    ) -> Self {
        let mtconfig = Self::new(meta, cols);
        meta.enable_equality(mtconfig.aux);
        mtconfig.configure(meta, rtable, imtable, configure);
        mtconfig
    }
}

pub struct MemoryTableChip<F: FieldExt> {
    config: MemoryTableConfig<F>,
}

impl<F: FieldExt> MemoryTableChip<F> {
    pub fn new(config: MemoryTableConfig<F>) -> Self {
        MemoryTableChip { config }
    }

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        mtable: &MTable,
        etable_rest_mops_cell: Option<Cell>,
        consecutive_zero_offset: u64,
    ) -> Result<(), Error> {
        assert_eq!(mtable_rows() % (STEP_SIZE as usize), 0);

        for i in 0..mtable_rows() {
            ctx.region
                .assign_fixed(|| "mtable sel", self.config.sel, i, || Ok(F::one()))?;

            if i % (STEP_SIZE as usize) == 0 {
                ctx.region.assign_fixed(
                    || "block_first_line_sel",
                    self.config.block_first_line_sel,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i >= STEP_SIZE as usize {
                ctx.region.assign_fixed(
                    || "following_block_sel",
                    self.config.following_block_sel,
                    i,
                    || Ok(F::one()),
                )?;
            }
        }

        let rest_mops_cell = ctx.region.assign_advice(
            || "rest mops",
            self.config.aux,
            RotationOfAuxColumn::RestMops as usize,
            || Ok(F::from(0u64)),
        )?;
        if let Some(etable_rest_mops_cell) = etable_rest_mops_cell {
            ctx.region
                .constrain_equal(rest_mops_cell.cell(), etable_rest_mops_cell)?;
        }

        let mut mops = mtable
            .entries()
            .iter()
            .fold(0, |acc, e| acc + if e.atype.is_init() { 0 } else { 1 });

        let mut last_entry: Option<&MemoryTableEntry> = None;
        for (index, entry) in mtable.entries().iter().enumerate() {
            macro_rules! assign_advice {
                ($key: expr, $offset: expr, $column: ident, $value: expr) => {
                    ctx.region.assign_advice(
                        || $key,
                        self.config.$column,
                        index * (STEP_SIZE as usize) + ($offset as usize),
                        || Ok($value),
                    )?
                };
            }

            macro_rules! assign_row_diff {
                ($offset: expr, $column: ident) => {
                    self.config.index.assign(
                        ctx,
                        Some($offset as usize + index * STEP_SIZE as usize),
                        (entry.$column as u64).into(),
                        (F::from(entry.$column as u64)
                            - F::from(
                                last_entry
                                    .as_ref()
                                    .map(|x| x.$column as u64)
                                    .unwrap_or(0u64),
                            )),
                    )?;
                };
            }

            // enable column
            {
                assign_advice!("enable", 0, bit, F::one());
                assign_advice!(
                    "vtype is i64",
                    RotationOfBitColumn::Is64Bit,
                    bit,
                    if entry.vtype == VarType::I64 {
                        F::one()
                    } else {
                        F::zero()
                    }
                );
                assign_advice!(
                    "ltype is stack",
                    RotationOfBitColumn::IsStack,
                    bit,
                    if entry.ltype == LocationType::Stack {
                        F::one()
                    } else {
                        F::zero()
                    }
                );
                assign_advice!(
                    "is mutable",
                    RotationOfBitColumn::IsMutable,
                    bit,
                    F::from(entry.is_mutable)
                );

                if (entry.ltype == LocationType::Heap || entry.ltype == LocationType::Global)
                    && entry.atype.is_positive_init()
                {
                    assign_advice!(
                        "vtype imtable selector",
                        RotationOfBitColumn::IMTableSelectorStart as i32
                            + entry.offset as i32 % (IMTABLE_COLOMNS as i32),
                        bit,
                        F::one()
                    );
                }
            }

            // index column
            {
                assign_row_diff!(RotationOfIndexColumn::LTYPE, ltype);
                assign_row_diff!(RotationOfIndexColumn::MMID, mmid);
                assign_row_diff!(RotationOfIndexColumn::OFFSET, offset);
                assign_row_diff!(RotationOfIndexColumn::EID, eid);
                assign_row_diff!(RotationOfIndexColumn::EMID, emid);

                for i in (RotationOfIndexColumn::MAX as i32)..STEP_SIZE {
                    self.config.index.assign(
                        ctx,
                        Some(index * STEP_SIZE as usize + i as usize),
                        F::zero(),
                        F::zero(),
                    )?;
                }
            }

            // aux column
            {
                let mut same_ltype = false;
                let mut same_mmid = false;
                let mut same_offset = false;
                let mut same_eid = false;

                if let Some(last_entry) = last_entry {
                    same_ltype = last_entry.ltype == entry.ltype;
                    same_mmid = last_entry.mmid == entry.mmid && same_ltype;
                    same_offset = last_entry.offset == entry.offset && same_mmid;
                    same_eid = last_entry.eid == entry.eid && same_offset;
                }

                assign_advice!(
                    "constant 1",
                    RotationOfAuxColumn::ConstantOne,
                    aux,
                    F::one()
                );
                assign_advice!(
                    "same ltype",
                    RotationOfAuxColumn::SameLtype,
                    aux,
                    F::from(same_ltype as u64)
                );
                assign_advice!(
                    "same mmid",
                    RotationOfAuxColumn::SameMmid,
                    aux,
                    F::from(same_mmid as u64)
                );
                assign_advice!(
                    "same offset",
                    RotationOfAuxColumn::SameOffset,
                    aux,
                    F::from(same_offset as u64)
                );
                assign_advice!(
                    "same eid",
                    RotationOfAuxColumn::SameEid,
                    aux,
                    F::from(same_eid as u64)
                );
                assign_advice!(
                    "atype",
                    RotationOfAuxColumn::Atype,
                    aux,
                    F::from(entry.atype.into_index())
                );
                assign_advice!(
                    "rest mops",
                    RotationOfAuxColumn::RestMops,
                    aux,
                    F::from(mops)
                );

                if let AccessType::Init(InitType::Lazy) = entry.atype {
                    assert!(entry.offset >= consecutive_zero_offset);

                    assign_advice!(
                        "lazy init helper",
                        RotationOfAuxColumn::RangeInLazyInitDiff,
                        aux,
                        F::from(entry.offset - consecutive_zero_offset)
                    );

                    assign_advice!(
                        "lazy init helper",
                        RotationOfBitColumn::IsLazyInit,
                        bit,
                        F::from(1)
                    );
                }
            }

            // bytes column
            {
                let mut bytes = Vec::from(entry.value.to_le_bytes());
                bytes.resize(8, 0);
                for i in 0..8 {
                    assign_advice!("bytes", i, bytes, F::from(bytes[i] as u64));
                }
            }

            if !entry.atype.is_init() {
                mops -= 1;
            }

            last_entry = Some(entry);
            ctx.offset += STEP_SIZE as usize;
        }

        if let Some(last_entry) = last_entry {
            self.config
                .index
                .assign(ctx, None, F::zero(), -F::from(last_entry.ltype as u64))?;
            ctx.offset += 1;
            self.config
                .index
                .assign(ctx, None, F::zero(), -F::from(last_entry.mmid as u64))?;
            ctx.offset += 1;
            self.config
                .index
                .assign(ctx, None, F::zero(), -F::from(last_entry.offset as u64))?;
            ctx.offset += 1;
            self.config
                .index
                .assign(ctx, None, F::zero(), -F::from(last_entry.eid as u64))?;
            ctx.offset += 1;
            self.config
                .index
                .assign(ctx, None, F::zero(), -F::from(last_entry.emid as u64))?;
            ctx.offset += 1;
        }

        for i in ctx.offset..max_mtable_rows() as usize {
            self.config
                .index
                .assign(ctx, Some(i), F::zero(), F::zero())?;
        }

        Ok(())
    }
}
