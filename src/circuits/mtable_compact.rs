use self::configure::MemoryTableConstriants;
use super::imtable::InitMemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::utils::row_diff::RowDiffConfig;
use super::utils::Context;
use crate::circuits::mtable_compact::configure::STEP_SIZE;
use crate::circuits::mtable_compact::expression::RotationAux;
use crate::circuits::mtable_compact::expression::RotationIndex;
use crate::circuits::mtable_compact::expression::ROTATION_VTYPE_GE_EIGHT_BYTES;
use crate::circuits::mtable_compact::expression::ROTATION_VTYPE_GE_FOUR_BYTES;
use crate::circuits::mtable_compact::expression::ROTATION_VTYPE_GE_TWO_BYTES;
use crate::circuits::mtable_compact::expression::ROTATION_VTYPE_SIGN;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use specs::mtable::AccessType;
use specs::mtable::MTable;
use specs::mtable::MemoryTableEntry;
use std::marker::PhantomData;

const MAX_MATBLE_ROWS: usize = 1usize << 15;
const MTABLE_ROWS: usize = MAX_MATBLE_ROWS / STEP_SIZE as usize * STEP_SIZE as usize;

lazy_static! {
    static ref VAR_TYPE_SHIFT: BigUint = BigUint::from(1u64) << 64;
    static ref ACCESS_TYPE_SHIFT: BigUint = BigUint::from(1u64) << 77;
    static ref LOC_TYPE_SHIFT: BigUint = BigUint::from(1u64) << 79;
    static ref OFFSET_SHIFT: BigUint = BigUint::from(1u64) << 80;
    static ref MMID_SHIFT: BigUint = BigUint::from(1u64) << 96;
    static ref EMID_SHIFT: BigUint = BigUint::from(1u64) << 112;
    static ref EID_SHIFT: BigUint = BigUint::from(1u64) << 128;
}

pub mod configure;
pub mod expression;

#[derive(Clone)]
pub struct MemoryTableConfig<F: FieldExt> {
    pub(crate) sel: Column<Fixed>,
    pub(crate) following_block_sel: Column<Fixed>,
    pub(crate) block_first_line_sel: Column<Fixed>,

    // Rotation
    // 0: enable
    // 1: vtype ge 2 bytes
    // 2: vtype ge 4 bytes
    // 3: vtype ge 8 bytes
    // 4: sign mask
    pub(crate) bit: Column<Advice>,

    // Rotation
    // 0: ltype  (Stack, Heap)
    // 1: mmid   (0 for Stack,  1 .. for Heap)
    // 2: offset (sp for Stack, address for Heap)
    // 3: eid
    // 4: emid
    pub(crate) index: RowDiffConfig<F>,

    // Rotation:
    // 0: constant 1
    // 1: same ltype
    // 2: same mmid
    // 3: same offset
    // 4: same eid
    // 5: atype
    // 6: rest mops
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
    ) -> Self {
        let mtconfig = Self::new(meta, cols);
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
        etable_rest_mops_cell: Cell,
    ) -> Result<(), Error> {
        assert_eq!(MTABLE_ROWS % (STEP_SIZE as usize), 0);

        for i in 0..MTABLE_ROWS {
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

        let mut mops = mtable.entries().iter().fold(0, |acc, e| {
            acc + if e.atype == AccessType::Init { 0 } else { 1 }
        });

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
                    "vtype ge two bytes",
                    ROTATION_VTYPE_GE_TWO_BYTES,
                    bit,
                    if entry.vtype.byte_size() >= 2 {
                        F::one()
                    } else {
                        F::zero()
                    }
                );
                assign_advice!(
                    "vtype ge four bytes",
                    ROTATION_VTYPE_GE_FOUR_BYTES,
                    bit,
                    if entry.vtype.byte_size() >= 4 {
                        F::one()
                    } else {
                        F::zero()
                    }
                );
                assign_advice!(
                    "vtype ge eight bytes",
                    ROTATION_VTYPE_GE_EIGHT_BYTES,
                    bit,
                    if entry.vtype.byte_size() >= 8 {
                        F::one()
                    } else {
                        F::zero()
                    }
                );
                assign_advice!(
                    "vtype sign",
                    ROTATION_VTYPE_SIGN,
                    bit,
                    if entry.vtype as usize & 1 == 1 {
                        F::one()
                    } else {
                        F::zero()
                    }
                );
            }

            // index column
            {
                assign_row_diff!(RotationIndex::LTYPE, ltype);
                assign_row_diff!(RotationIndex::MMID, mmid);
                assign_row_diff!(RotationIndex::OFFSET, offset);
                assign_row_diff!(RotationIndex::EID, eid);
                assign_row_diff!(RotationIndex::EMID, emid);

                for i in (RotationIndex::MAX as i32)..STEP_SIZE {
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

                assign_advice!("constant 1", RotationAux::ConstantOne, aux, F::one());
                assign_advice!(
                    "same ltype",
                    RotationAux::SameLtype,
                    aux,
                    F::from(same_ltype as u64)
                );
                assign_advice!(
                    "same mmid",
                    RotationAux::SameMmid,
                    aux,
                    F::from(same_mmid as u64)
                );
                assign_advice!(
                    "same offset",
                    RotationAux::SameOffset,
                    aux,
                    F::from(same_offset as u64)
                );
                assign_advice!(
                    "same eid",
                    RotationAux::SameEid,
                    aux,
                    F::from(same_eid as u64)
                );
                assign_advice!(
                    "atype",
                    RotationAux::Atype,
                    aux,
                    F::from(entry.atype as u64)
                );
                let cell = assign_advice!("rest mops", RotationAux::RestMops, aux, F::from(mops));
                if index == 0 {
                    ctx.region
                        .constrain_equal(cell.cell(), etable_rest_mops_cell)?;
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
                ctx.offset += 1;
                self.config
                    .index
                    .assign(ctx, None, F::zero(), -F::from(last_entry.mmid as u64))?;
                ctx.offset += 1;
                self.config.index.assign(
                    ctx,
                    None,
                    F::zero(),
                    -F::from(last_entry.offset as u64),
                )?;
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
        }

        println!("offset {}", ctx.offset);

        for i in ctx.offset..MAX_MATBLE_ROWS {
            self.config
                .index
                .assign(ctx, Some(i), F::zero(), F::zero())?;
        }

        Ok(())
    }
}
