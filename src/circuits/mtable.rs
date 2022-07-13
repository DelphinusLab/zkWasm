use super::imtable::InitMemoryTableConfig;
use super::rtable::RangeTableConfig;
use super::utils::bn_to_field;
use super::utils::row_diff::RowDiffConfig;
use super::utils::Context;
use crate::constant;
use crate::constant_from;
use crate::curr;
use crate::next;
use crate::prev;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::mtable::AccessType;
use specs::mtable::LocationType;
use specs::mtable::MemoryTableEntry;
use std::marker::PhantomData;

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
    eid: RowDiffConfig<F>,
    emid: RowDiffConfig<F>,
    mmid: RowDiffConfig<F>,
    offset: RowDiffConfig<F>,
    ltype: RowDiffConfig<F>,
    atype: Column<Advice>,
    vtype: Column<Advice>,
    value: Column<Advice>,
    enable: Column<Advice>,
    same_location: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
    ) -> Self {
        let mtconfig = Self::new(meta, cols);

        mtconfig.configure_enable(meta);
        mtconfig.configure_sort(meta, rtable);
        mtconfig.configure_stack_or_heap(meta);
        mtconfig.configure_range(meta, rtable);
        mtconfig.configure_same_location(meta);
        mtconfig.configure_rule(meta, imtable);

        mtconfig
    }

    pub fn configure_stack_read_in_table(
        &self,
        key: &'static str,
        key_rev: &'static str,
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
                self.encode_for_lookup(meta) * enable(meta),
            )]
        });

        meta.lookup_any(key_rev, |meta| {
            vec![(
                self.encode_for_lookup(meta) * enable(meta),
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
            )]
        });
    }

    pub fn configure_stack_write_in_table(
        &self,
        key: &'static str,
        key_rev: &'static str,
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
                self.encode_for_lookup(meta) * enable(meta),
            )]
        });

        meta.lookup_any(key_rev, |meta| {
            vec![(
                self.encode_for_lookup(meta) * enable(meta),
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
            )]
        });
    }
}

impl<F: FieldExt> MemoryTableConfig<F> {
    fn encode_for_lookup(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.eid.data(meta) * constant!(bn_to_field(&EID_SHIFT))
            + self.emid.data(meta) * constant!(bn_to_field(&EMID_SHIFT))
            + self.mmid.data(meta) * constant!(bn_to_field(&MMID_SHIFT))
            + self.offset.data(meta) * constant!(bn_to_field(&OFFSET_SHIFT))
            + self.ltype.data(meta) * constant!(bn_to_field(&LOC_TYPE_SHIFT))
            + curr!(meta, self.atype) * constant!(bn_to_field(&ACCESS_TYPE_SHIFT))
            + curr!(meta, self.vtype) * constant!(bn_to_field(&VAR_TYPE_SHIFT))
            + curr!(meta, self.value)
    }

    fn new(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let emid = RowDiffConfig::configure("mtable emid", meta, cols, |_| constant_from!(1));
        let ltype = RowDiffConfig::configure("mtable ltype", meta, cols, |_| constant_from!(1));
        let mmid = RowDiffConfig::configure("mtable mmid", meta, cols, |_| constant_from!(1));
        let offset = RowDiffConfig::configure("mtable offset", meta, cols, |_| constant_from!(1));
        let eid = RowDiffConfig::configure("mtable eid", meta, cols, |_| constant_from!(1));
        let value = cols.next().unwrap();
        let atype = cols.next().unwrap();
        let vtype = cols.next().unwrap();
        let enable = cols.next().unwrap();
        let same_location = cols.next().unwrap();

        MemoryTableConfig {
            _mark: PhantomData,
            ltype,
            mmid,
            offset,
            eid,
            emid,
            atype,
            vtype,
            value,
            enable,
            same_location,
        }
    }

    fn configure_enable(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("enable seq", |meta| {
            let curr = curr!(meta, self.enable);
            let next = next!(meta, self.enable);
            vec![
                next * (curr.clone() - Expression::<F>::Constant(F::one())),
                curr.clone() * (curr.clone() - Expression::<F>::Constant(F::one())),
            ]
        });
    }

    fn configure_same_location(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("is same location", |meta| {
            let same_loc = curr!(meta, self.same_location);
            vec![
                self.ltype.is_same(meta) * self.mmid.is_same(meta) * self.offset.is_same(meta)
                    - same_loc,
            ]
        })
    }

    fn configure_stack_or_heap(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("is same location", |meta| {
            let ltype = self.ltype.data(meta);
            vec![ltype.clone() * (ltype - Expression::Constant(F::one()))]
        })
    }

    fn configure_range(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        rtable.configure_in_common_range(meta, "mmid in range", |meta| self.mmid.data(meta));
        rtable.configure_in_common_range(meta, "offset in range", |meta| self.offset.data(meta));
        rtable.configure_in_common_range(meta, "eid in range", |meta| self.eid.data(meta));
        rtable.configure_in_common_range(meta, "emid in range", |meta| self.emid.data(meta));
        rtable.configure_in_common_range(meta, "vtype in range", |meta| self.emid.data(meta));
    }

    fn configure_sort(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        rtable.configure_in_common_range(meta, "ltype sort", |meta| {
            self.is_enable(meta) * self.ltype.diff(meta)
        });

        rtable.configure_in_common_range(meta, "mmid sort", |meta| {
            self.is_enable(meta) * self.ltype.is_same(meta) * self.mmid.diff(meta)
        });
        rtable.configure_in_common_range(meta, "offset sort", |meta| {
            self.is_enable(meta)
                * self.ltype.is_same(meta)
                * self.mmid.is_same(meta)
                * self.offset.diff(meta)
        });
        rtable.configure_in_common_range(meta, "eid sort", |meta| {
            self.is_enable(meta) * self.is_same_location(meta) * self.eid.diff(meta)
        });
        rtable.configure_in_common_range(meta, "emid sort", |meta| {
            self.is_enable(meta)
                * self.is_same_location(meta)
                * self.eid.is_same(meta)
                * self.emid.diff(meta)
        });
    }

    fn configure_rule(&self, meta: &mut ConstraintSystem<F>, imtable: &InitMemoryTableConfig<F>) {
        meta.create_gate("mtable read after write", |meta| {
            vec![
                self.is_enable(meta) * self.is_read_not_bit(meta) * self.diff(meta, self.value),
                self.is_enable(meta) * self.is_read_not_bit(meta) * self.diff(meta, self.vtype),
            ]
        });

        meta.create_gate("mtable emid uniqe", |meta| {
            vec![self.is_enable(meta) * self.is_same_location(meta) * self.emid.is_same(meta)]
        });

        meta.create_gate("mtable stack first line", |meta| {
            vec![
                self.is_enable(meta)
                    * (self.is_same_location(meta) - Expression::Constant(F::one()))
                    * self.is_stack(meta)
                    * (curr!(meta, self.atype) - constant_from!(AccessType::Write)),
            ]
        });

        imtable.configure_in_table(meta, "mtable heap first line", |meta| {
            self.is_enable(meta)
                * (Expression::Constant(F::one()) - self.is_same_location(meta))
                * self.is_heap(meta)
                * imtable.encode(
                    self.mmid.data(meta),
                    self.offset.data(meta),
                    curr!(meta, self.value),
                )
        })
    }

    fn is_heap(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        Expression::Constant(F::one()) - self.ltype.data(meta)
    }

    fn is_stack(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        self.ltype.data(meta)
    }

    fn diff(&self, meta: &mut VirtualCells<F>, col: Column<Advice>) -> Expression<F> {
        curr!(meta, col) - prev!(meta, col)
    }

    fn is_enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.enable)
    }

    fn is_same_location(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.same_location)
    }

    fn is_read_not_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let atype = curr!(meta, self.atype);
        (atype.clone() - constant_from!(AccessType::Init))
            * (atype - constant_from!(AccessType::Write))
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
    ) -> Result<(), Error> {
        let mut last_entry: Option<&MemoryTableEntry> = None;
        for entry in entries {
            macro_rules! row_diff_assign {
                ($x: ident) => {
                    self.config.$x.assign(
                        ctx,
                        (entry.$x as u64).into(),
                        ((entry.$x as u64)
                            - last_entry.as_ref().map(|x| x.$x as u64).unwrap_or(0u64))
                        .into(),
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

            ctx.region.assign_advice(
                || "mtable vtype",
                self.config.vtype,
                ctx.offset,
                || Ok((entry.vtype as u64).into()),
            )?;

            ctx.region.assign_advice(
                || "mtable value",
                self.config.value,
                ctx.offset,
                || Ok((entry.value as u64).into()),
            )?;

            ctx.region.assign_advice(
                || "mtable enable",
                self.config.enable,
                ctx.offset,
                || Ok(F::one()),
            )?;

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

            last_entry = Some(entry);
            ctx.next();
        }
        Ok(())
    }
}
