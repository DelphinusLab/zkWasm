use super::JumpTableConfig;
use crate::{
    circuits::{rtable::RangeTableConfig, utils::bn_to_field},
    constant, constant_from, curr, fixed_curr, fixed_prev,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};
use num_bigint::BigUint;

const EID_SHIFT: usize = 64;
const LAST_JUMP_EID_SHIFT: usize = 48;
const MOID_SHIFT: usize = 32;
const FID_SHIFT: usize = 16;

pub trait JTableConstraint<F: FieldExt> {
    fn configure(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        self.enable_rest_jops_permutation(meta);
        self.configure_rest_jops_decrease(meta);
        self.configure_final_rest_jops_zero(meta);
        self.configure_rest_jops_in_u16_range(meta, rtable);
    }

    fn enable_rest_jops_permutation(&self, meta: &mut ConstraintSystem<F>);
    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>);
    fn configure_final_rest_jops_zero(&self, meta: &mut ConstraintSystem<F>);
    fn configure_rest_jops_in_u16_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    );
}

impl<F: FieldExt> JTableConstraint<F> for JumpTableConfig<F> {
    fn enable_rest_jops_permutation(&self, meta: &mut ConstraintSystem<F>) {
        meta.enable_equality(self.data);
    }

    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("jtable rest decrease", |meta| {
            vec![
                (self.rest(meta) - self.next_rest(meta) - constant_from!(2))
                    * self.rest(meta)
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn configure_final_rest_jops_zero(&self, meta: &mut ConstraintSystem<F>) {
        // (entry == 0 -> rest == 0)
        // <-> (exists aux, entry * aux == rest)
        meta.create_gate("jtable is zero at end", |meta| {
            vec![
                (self.entry(meta) * self.aux(meta) - self.rest(meta)) * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn configure_rest_jops_in_u16_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    ) {
        rtable.configure_in_common_range(meta, "jtable rest in common range", |meta| {
            curr!(meta, self.data) * fixed_curr!(meta, self.sel)
        });
    }
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
        eid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        last_jump_eid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        moid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        fid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
        iid: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        let one = BigUint::from(1u64);
        meta.lookup_any("jtable lookup", |meta| {
            vec![(
                enable(meta)
                    * (eid(meta) * constant!(bn_to_field(&(&one << EID_SHIFT)))
                        + last_jump_eid(meta)
                            * constant!(bn_to_field(&(&one << LAST_JUMP_EID_SHIFT)))
                        + moid(meta) * constant!(bn_to_field(&(&one << MOID_SHIFT)))
                        + fid(meta) * constant!(bn_to_field(&(&one << FID_SHIFT)))
                        + iid(meta)),
                curr!(meta, self.data) * fixed_prev!(meta, self.sel),
            )]
        });
    }

    pub(super) fn new(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let sel = meta.fixed_column();
        let data = cols.next().unwrap();

        JumpTableConfig {
            sel,
            data,
            _m: std::marker::PhantomData,
        }
    }
}
