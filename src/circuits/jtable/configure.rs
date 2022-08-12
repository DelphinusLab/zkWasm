use super::JumpTableConfig;
use crate::{
    circuits::{rtable::RangeTableConfig, Lookup},
    constant_from, fixed_curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, ConstraintSystem, Expression, VirtualCells},
};

pub trait JTableConstraint<F: FieldExt> {
    fn configure(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        self.enable_is_bit(meta);
        self.enable_rest_jops_permutation(meta);
        self.configure_rest_jops_decrease(meta);
        // self.disabled_block_should_be_empty(meta);
        self.configure_rest_jops_in_u16_range(meta, rtable);
    }

    fn enable_rest_jops_permutation(&self, meta: &mut ConstraintSystem<F>);
    fn enable_is_bit(&self, meta: &mut ConstraintSystem<F>);
    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>);
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

    fn enable_is_bit(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("enable is bit", |meta| {
            vec![
                self.enable(meta)
                    * (self.enable(meta) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("jtable rest decrease", |meta| {
            vec![
                (self.rest(meta) - self.next_rest(meta) - constant_from!(2))
                    * self.enable(meta)
                    * fixed_curr!(meta, self.sel),
                (self.rest(meta) - self.next_rest(meta))
                    * (self.enable(meta) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn configure_rest_jops_in_u16_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    ) {
        rtable.configure_in_common_range(meta, "jtable rest in common range", |meta| {
            self.rest(meta) * fixed_curr!(meta, self.sel)
        });
    }
}

impl<F: FieldExt> Lookup<F> for JumpTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                expr(meta),
                self.entry(meta) * self.enable(meta) * fixed_curr!(meta, self.sel),
            )]
        });
    }

    fn encode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        self.entry(meta) * self.enable(meta) * fixed_curr!(meta, self.sel)
    }
}

impl<F: FieldExt> JumpTableConfig<F> {
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
