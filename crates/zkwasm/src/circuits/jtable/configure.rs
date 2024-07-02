use super::JumpTableConfig;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

pub trait JTableConstraint<F: FieldExt> {
    fn configure(&self, meta: &mut ConstraintSystem<F>, is_last_slice: bool) {
        self.enable_returned_are_bit(meta);
        self.enable_permutation(meta);
        self.configure_rest_jops_decrease(meta);
        self.disabled_block_should_be_end(meta, is_last_slice);
        self.disabled_block_has_no_entry_value(meta);
    }

    fn enable_permutation(&self, meta: &mut ConstraintSystem<F>);
    fn enable_returned_are_bit(&self, meta: &mut ConstraintSystem<F>);
    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>);
    fn disabled_block_should_be_end(&self, meta: &mut ConstraintSystem<F>, is_last_slice: bool);
    fn disabled_block_has_no_entry_value(&self, meta: &mut ConstraintSystem<F>);
}

impl<F: FieldExt> JTableConstraint<F> for JumpTableConfig<F> {
    fn enable_permutation(&self, meta: &mut ConstraintSystem<F>) {
        meta.enable_equality(self.value);
    }

    fn enable_returned_are_bit(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("enable and returned are bit", |meta| {
            vec![
                self.enable(meta) * (self.enable(meta) - constant_from!(1)) * self.sel(meta),
                self.returned(meta) * (self.returned(meta) - constant_from!(1)) * self.sel(meta),
            ]
        });
    }

    fn configure_rest_jops_decrease(&self, meta: &mut ConstraintSystem<F>) {
        /*
         * Why we do not need `enable == 1 -> encode != 0`.
         *   If enable == 1 but encode == 0, it means the number of ops may greater than the number of encoding. However
         *   - If the number of ops is not correct, the equality between etable and frame table will fail.
         *   - If the number of ops is correct, encode == 0 implies an entry is missing and etable cannot
         *     lookup the correct entry in frame table.
         */
        meta.create_gate("c3. jtable rest decrease", |meta| {
            vec![
                (self.rest_return_ops(meta)
                    - self.next_rest_return_ops(meta)
                    - self.returned(meta) * self.enable(meta))
                    * self.sel(meta),
                (self.rest_call_ops(meta) - self.next_rest_call_ops(meta) - self.enable(meta)
                    + self.inherited_bit(meta) * self.enable(meta))
                    * self.sel(meta),
            ]
        });
    }

    fn disabled_block_should_be_end(&self, meta: &mut ConstraintSystem<F>, is_last_slice: bool) {
        meta.create_gate("c5. jtable ends up", |meta| {
            vec![
                (constant_from!(1) - self.enable(meta))
                    * (constant_from!(1) - self.inherited_bit(meta))
                    * self.rest_call_ops(meta)
                    * self.sel(meta),
                (constant_from!(1) - self.enable(meta))
                    * (constant_from!(1) - self.inherited_bit(meta))
                    * self.rest_return_ops(meta)
                    * self.sel(meta),
            ]
        });

        if is_last_slice {
            meta.create_gate("c5. jtable ends up", |meta| {
                vec![(constant_from!(1) - self.returned(meta)) * self.enable(meta) * self.sel(meta)]
            });
        }
    }

    fn disabled_block_has_no_entry_value(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("c6. jtable entry is zero on disabled", |meta| {
            vec![
                (constant_from!(1) - self.enable(meta)) * self.encode(meta) * self.sel(meta),
                (constant_from!(1) - self.enable(meta)) * self.returned(meta) * self.sel(meta),
            ]
        });
    }
}

impl<F: FieldExt> JumpTableConfig<F> {
    /// Frame Table Constraint 4. Etable step's call/return record can be found on jtable_entry
    pub(in crate::circuits) fn configure_lookup_in_frame_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> (Expression<F>, Expression<F>, Expression<F>),
    ) {
        meta.lookup_any(key, |meta| {
            let (sel, is_returned_or_call, encode) = expr(meta);

            vec![
                (sel, self.sel(meta)),
                (is_returned_or_call, self.returned(meta)),
                (encode, self.encode(meta)),
            ]
        });
    }
}

impl<F: FieldExt> JumpTableConfig<F> {
    pub(super) fn new(meta: &mut ConstraintSystem<F>) -> Self {
        JumpTableConfig {
            sel: meta.fixed_column(),
            inherited: meta.fixed_column(),
            value: meta.advice_column(),
            _m: std::marker::PhantomData,
        }
    }
}
