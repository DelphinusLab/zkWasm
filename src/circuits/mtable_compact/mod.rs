use self::configure::MemoryTableConstriants;
use super::config::MAX_MATBLE_ROWS;
use super::imtable::InitMemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::shared_column_pool::SharedColumnPool;
use super::utils::row_diff::RowDiffConfig;
use super::utils::Context;
use crate::circuits::config::MTABLE_END_OFFSET;
use crate::circuits::mtable_compact::configure::STEP_SIZE;
use crate::circuits::IMTABLE_COLOMNS;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::mtable::MTable;
use specs::mtable::MemoryTableEntry;
use specs::mtable::VarType;
use std::marker::PhantomData;

const MTABLE_ROWS: usize = MAX_MATBLE_ROWS / STEP_SIZE as usize * STEP_SIZE as usize;

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
}

pub enum RotationOfBitColumn {
    Enable = 0,
    Is64Bit,
    IsStack,
    IsMutable,
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
        shared_column_pool: &SharedColumnPool<F>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
    ) -> Self {
        let mtconfig = Self::new(meta, shared_column_pool);
        meta.enable_equality(mtconfig.aux);
        mtconfig.configure(meta, rtable, imtable);
        mtconfig
    }
}

pub struct MemoryTableChip<F: FieldExt> {
    config: MemoryTableConfig<F>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> MemoryTableChip<F> {
    pub fn new(config: MemoryTableConfig<F>) -> Self {
        MemoryTableChip {
            config,
            _phantom: PhantomData,
        }
    }

    pub fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        mtable: &MTable,
        etable_rest_mops_cell: Option<Cell>,
    ) -> Result<(), Error> {
        assert_eq!(MTABLE_ROWS % (STEP_SIZE as usize), 0);
        assert_eq!(ctx.start_offset % (STEP_SIZE as usize), 0);

        for i in 0..MTABLE_ROWS {
            ctx.region.as_ref().borrow_mut().assign_fixed(
                || "mtable sel",
                self.config.sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if ctx.offset % (STEP_SIZE as usize) == 0 {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "block_first_line_sel",
                    self.config.block_first_line_sel,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            if i >= STEP_SIZE as usize {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "following_block_sel",
                    self.config.following_block_sel,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            ctx.next();
        }

        ctx.reset();

        let mut mops = mtable.entries().iter().fold(0, |acc, e| {
            acc + if e.atype == AccessType::Init { 0 } else { 1 }
        });

        let mut last_entry: Option<&MemoryTableEntry> = None;
        for entry in mtable.entries().iter() {
            macro_rules! assign_advice {
                ($key: expr, $offset: expr, $column: ident, $value: expr) => {
                    ctx.region.as_ref().borrow_mut().assign_advice(
                        || $key,
                        self.config.$column,
                        ctx.offset + ($offset as usize),
                        || Ok($value),
                    )?
                };
            }

            macro_rules! assign_row_diff {
                ($offset: expr, $column: ident) => {
                    self.config.index.assign(
                        ctx,
                        Some(ctx.offset + $offset as usize),
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
                    && entry.atype == AccessType::Init
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
                        Some(ctx.offset + i as usize),
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
                    F::from(entry.atype as u64)
                );
                let cell = assign_advice!(
                    "rest mops",
                    RotationOfAuxColumn::RestMops,
                    aux,
                    F::from(mops)
                );
                if ctx.offset == ctx.start_offset && etable_rest_mops_cell.is_some() {
                    ctx.region
                        .as_ref()
                        .borrow_mut()
                        .constrain_equal(cell.cell(), etable_rest_mops_cell.unwrap())?;
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

            if entry.atype != AccessType::Init {
                mops -= 1;
            }

            last_entry = Some(entry);
            ctx.offset += STEP_SIZE as usize;
        }

        match last_entry {
            None => {}
            Some(last_entry) => {
                self.config.index.assign(
                    ctx,
                    None,
                    F::zero(),
                    -F::from(last_entry.ltype as u64),
                )?;
                ctx.next();
                self.config
                    .index
                    .assign(ctx, None, F::zero(), -F::from(last_entry.mmid as u64))?;
                ctx.next();
                self.config.index.assign(
                    ctx,
                    None,
                    F::zero(),
                    -F::from(last_entry.offset as u64),
                )?;
                ctx.next();
                self.config
                    .index
                    .assign(ctx, None, F::zero(), -F::from(last_entry.eid as u64))?;
                ctx.next();
                self.config
                    .index
                    .assign(ctx, None, F::zero(), -F::from(last_entry.emid as u64))?;
                ctx.next();
            }
        }

        for i in ctx.offset..MTABLE_END_OFFSET {
            self.config
                .index
                .assign(ctx, Some(i), F::zero(), F::zero())?;
        }

        Ok(())
    }
}
