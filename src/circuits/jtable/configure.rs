use super::JumpTableConfig;
use crate::circuits::Lookup;
use crate::constant_from;
use crate::fixed_curr;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

pub trait JTableConstraint<F: FieldExt> {
    fn configure(&self, meta: &mut ConstraintSystem<F>) {
        self.enable_is_bit(meta);
        self.enable_rest_jops_permutation(meta);
        self.configure_rest_jops_decrease(meta);
        self.disabled_block_should_be_empty(meta);
    }

    fn enable_rest_jops_permutation(&self, meta: &mut ConstraintSystem<F>);
    fn enable_is_bit(&self, meta: &mut ConstraintSystem<F>);
    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>);
    fn disabled_block_should_be_empty(&self, meta: &mut ConstraintSystem<F>);
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
        meta.create_gate("c3. jtable rest decrease", |meta| {
            vec![
                (self.rest(meta) - self.next_rest(meta) - constant_from!(2)
                    + self.static_bit(meta))
                    * self.enable(meta)
                    * fixed_curr!(meta, self.sel),
                (self.rest(meta) - self.next_rest(meta))
                    * (self.enable(meta) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn disabled_block_should_be_empty(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("c5. jtable ends up", |meta| {
            vec![
                (constant_from!(1) - self.enable(meta))
                    * self.rest(meta)
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }
}

impl<F: FieldExt> Lookup<F> for JumpTableConfig<F> {
    /// Frame Table Constraint 4. Etable step's call/return record can be found on jtable_entry
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
        let static_bit = meta.fixed_column();
        let data = cols.next().unwrap();

        JumpTableConfig {
            sel,
            static_bit,
            data,
            _m: std::marker::PhantomData,
        }
    }
}
