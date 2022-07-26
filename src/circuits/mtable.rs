use super::imtable::InitMemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::utils::bn_to_field;
use super::utils::row_diff::RowDiffConfig;
use super::utils::tvalue::TValueConfig;
use super::utils::Context;
use crate::constant;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::mtable::MemoryTableEntry;
use std::marker::PhantomData;

pub mod configure;
pub mod expression;

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

#[derive(Clone)]
pub struct MemoryTableConfig<F: FieldExt> {
    sel: Column<Fixed>,
    eid: RowDiffConfig<F>,
    emid: RowDiffConfig<F>,

    ltype: RowDiffConfig<F>,
    mmid: RowDiffConfig<F>,
    offset: RowDiffConfig<F>,
    same_location: Column<Advice>,

    atype: Column<Advice>,
    tvalue: TValueConfig<F>,

    rest_mops: Column<Advice>,
    enable: Column<Advice>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
    ) -> Self {
        let mtconfig = Self::new(meta, cols, rtable);
        mtconfig.configure_enable(meta);
        mtconfig.configure_range(meta, rtable);
        mtconfig.configure_sort(meta, rtable);
        mtconfig.configure_same_location(meta);
        mtconfig.configure_rule(meta, imtable);
        mtconfig
    }

    pub fn configure_stack_read_in_table(
        &self,
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        eid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        emid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        sp: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        vtype: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                (eid(meta) * constant!(bn_to_field(&EID_SHIFT))
                    + emid(meta) * constant!(bn_to_field(&EMID_SHIFT))
                    + sp(meta) * constant!(bn_to_field(&OFFSET_SHIFT))
                    + constant!(bn_to_field(&LOC_TYPE_SHIFT))
                        * constant_from!(LocationType::Stack)
                    + constant!(bn_to_field(&ACCESS_TYPE_SHIFT))
                        * constant_from!(AccessType::Read)
                    + vtype(meta) * constant!(bn_to_field(&VAR_TYPE_SHIFT))
                    + value(meta))
                    * enable(meta),
                self.encode_for_lookup(meta),
            )]
        });
    }

    pub fn configure_stack_write_in_table(
        &self,
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        eid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        emid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        sp: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        vtype: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                (eid(meta) * constant!(bn_to_field(&EID_SHIFT))
                    + emid(meta) * constant!(bn_to_field(&EMID_SHIFT))
                    + sp(meta) * constant!(bn_to_field(&OFFSET_SHIFT))
                    + constant!(bn_to_field(&LOC_TYPE_SHIFT))
                        * constant_from!(LocationType::Stack)
                    + constant!(bn_to_field(&ACCESS_TYPE_SHIFT))
                        * constant_from!(AccessType::Write)
                    + vtype(meta) * constant!(bn_to_field(&VAR_TYPE_SHIFT))
                    + value(meta))
                    * enable(meta),
                self.encode_for_lookup(meta),
            )]
        });
    }

    pub fn configure_memory_load_in_table(
        &self,
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        eid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        emid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        mmid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        address: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        vtype: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        block_value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                (eid(meta) * constant!(bn_to_field(&EID_SHIFT))
                    + emid(meta) * constant!(bn_to_field(&EMID_SHIFT))
                    + mmid(meta) * constant!(bn_to_field(&MMID_SHIFT))
                    + address(meta) * constant!(bn_to_field(&OFFSET_SHIFT))
                    + constant!(bn_to_field(&LOC_TYPE_SHIFT)) * constant_from!(LocationType::Heap)
                    + constant!(bn_to_field(&ACCESS_TYPE_SHIFT))
                        * constant_from!(AccessType::Read)
                    + vtype(meta) * constant!(bn_to_field(&VAR_TYPE_SHIFT))
                    + block_value(meta))
                    * enable(meta),
                self.encode_for_lookup(meta),
            )]
        });
    }

    pub fn configure_memory_store_in_table(
        &self,
        key: &'static str,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        eid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        emid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        mmid: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        address: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        vtype: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        block_value: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                (eid(meta) * constant!(bn_to_field(&EID_SHIFT))
                    + emid(meta) * constant!(bn_to_field(&EMID_SHIFT))
                    + mmid(meta) * constant!(bn_to_field(&MMID_SHIFT))
                    + address(meta) * constant!(bn_to_field(&OFFSET_SHIFT))
                    + constant!(bn_to_field(&LOC_TYPE_SHIFT)) * constant_from!(LocationType::Heap)
                    + constant!(bn_to_field(&ACCESS_TYPE_SHIFT))
                        * constant_from!(AccessType::Write)
                    + vtype(meta) * constant!(bn_to_field(&VAR_TYPE_SHIFT))
                    + block_value(meta))
                    * enable(meta),
                self.encode_for_lookup(meta),
            )]
        });
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

        Ok(())
    }
}
