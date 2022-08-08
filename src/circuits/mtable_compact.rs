use self::configure::MemoryTableConstriants;
use self::configure::STEP_SIZE;
use super::imtable::InitMemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::utils::row_diff::RowDiffConfig;
use super::utils::Context;
use crate::curr;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Fixed;
use num_bigint::BigUint;
use specs::mtable::MemoryTableEntry;
use std::marker::PhantomData;

const MTABLE_ROWS: usize = 1usize << 16;

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
    pub(crate) enable: Column<Advice>,

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
    // 6: vtype
    // 7: rest mops
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
        rtable.configure_in_u8_range(meta, "mtable bytes", |meta| {
            curr!(meta, mtconfig.bytes) * mtconfig.enable_line(meta)
        });
        
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
        entries: &Vec<MemoryTableEntry>,
        etable_rest_mops_cell: Option<Cell>,
    ) -> Result<(), Error> {
        /*
            for i in 0..MTABLE_ROWS {
                ctx.region
                    .assign_fixed(|| "mtable sel", self.config.sel, i, || Ok(F::one()))?;
            }

            let mut mops = entries.iter().fold(0, |acc, e| {
                acc + if e.atype == AccessType::Init { 0 } else { 1 }
            });

            let mut last_entry: Option<&MemoryTableEntry> = None;
            for (i, entry) in entries.into_iter().enumerate() {
                macro_rules! row_diff_assign {
                    ($x: ident) => {
                        self.config.$x.assign(
                            ctx,
                            (entry.$x as u64).into(),
                            (F::from(entry.$x as u64)
                                - F::from(last_entry.as_ref().map(|x| x.$x as u64).unwrap_or(0u64))),
                        )?;
                    };
                }

                row_diff_assign!(eid);
                row_diff_assign!(emid);
                row_diff_assign!(mmid);
                row_diff_assign!(offset);
                row_diff_assign!(ltype);

                ctx.region.assign_advice(
                    || "mtable atype",
                    self.config.atype,
                    ctx.offset,
                    || Ok((entry.atype as u64).into()),
                )?;

                self.config.tvalue.assign(ctx, entry.vtype, entry.value)?;

                ctx.region.assign_advice(
                    || "mtable enable",
                    self.config.enable,
                    ctx.offset,
                    || Ok(F::one()),
                )?;

                let cell = ctx.region.assign_advice(
                    || "mtable enable",
                    self.config.rest_mops,
                    ctx.offset,
                    || Ok(F::from(mops)),
                )?;
                if i == 0 && etable_rest_mops_cell.is_some() {
                    ctx.region
                        .constrain_equal(cell.cell(), etable_rest_mops_cell.unwrap())?;
                }

                ctx.region.assign_advice(
                    || "mtable same_location",
                    self.config.same_location,
                    ctx.offset,
                    || {
                        Ok(last_entry.as_ref().map_or(F::zero(), |last_entry| {
                            if last_entry.is_same_location(&entry) {
                                F::one()
                            } else {
                                F::zero()
                            }
                        }))
                    },
                )?;

                if entry.atype != AccessType::Init {
                    mops -= 1;
                }
                last_entry = Some(entry);
                ctx.next();
            }
        */
        Ok(())
    }
}
