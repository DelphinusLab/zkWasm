use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

use crate::fixed_curr;
use crate::foreign::ForeignTableConfig;
use crate::instance_curr;

use super::WasmInputHelperTableConfig;

impl<F: FieldExt> WasmInputHelperTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let enable = meta.fixed_column();
        let index = meta.fixed_column();
        let input = meta.instance_column();

        WasmInputHelperTableConfig {
            enable,
            index,
            input,
            _mark: std::marker::PhantomData,
        }
    }
}

impl<F: FieldExt> ForeignTableConfig<F> for WasmInputHelperTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Vec<Expression<F>>,
    ) {
        meta.lookup_any(key, |meta| {
            let mut exprs = expr(meta);

            vec![
                (exprs.remove(0), fixed_curr!(meta, self.index)),
                (exprs.remove(0), instance_curr!(meta, self.input)),
            ]
        });
    }
}
