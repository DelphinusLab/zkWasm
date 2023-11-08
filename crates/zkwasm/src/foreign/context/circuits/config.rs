use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;

use crate::curr;
use crate::fixed_curr;
use crate::foreign::ForeignTableConfig;

use super::ContextContHelperTableConfig;

impl<F: FieldExt> ContextContHelperTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, from_zero_index: Column<Fixed>) -> Self {
        let input = meta.named_advice_column("context_input".to_string());
        let output = meta.named_advice_column("context_output".to_string());
        meta.enable_equality(input);
        meta.enable_equality(output);

        ContextContHelperTableConfig {
            from_zero_index,
            input,
            output,
            _mark: PhantomData,
        }
    }
}

impl<F: FieldExt> ForeignTableConfig<F> for ContextContHelperTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        _key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Vec<Expression<F>>,
    ) {
        meta.lookup_any("context_cont input lookup", |meta| {
            let mut exprs = expr(meta);

            vec![
                (exprs.remove(0), fixed_curr!(meta, self.from_zero_index)),
                (exprs.remove(0), curr!(meta, self.input)),
            ]
        });

        meta.lookup_any("context_cont output lookup", |meta| {
            let mut exprs = expr(meta);

            let _ = exprs.remove(0);
            let _ = exprs.remove(0);

            vec![
                (exprs.remove(0), fixed_curr!(meta, self.from_zero_index)),
                (exprs.remove(0), curr!(meta, self.output)),
            ]
        });
    }
}
