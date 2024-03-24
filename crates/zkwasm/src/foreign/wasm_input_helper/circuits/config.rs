use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::VirtualCells;

use crate::curr;
use crate::fixed_curr;
use crate::fixed_next;
use crate::foreign::wasm_input_helper::circuits::encode_input_lookup;
use crate::foreign::ForeignTableConfig;
use crate::instance_curr;

use super::WasmInputHelperTableConfig;

impl<F: FieldExt> WasmInputHelperTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, from_zero_index: Column<Fixed>) -> Self {
        let input = meta.instance_column();
        let lookup = meta.advice_column();

        meta.create_gate("wasm input helper lookup", |meta| {
            vec![
                fixed_curr!(meta, from_zero_index)
                    * curr!(meta, lookup)
                    * (curr!(meta, lookup)
                        - encode_input_lookup(
                            fixed_next!(meta, from_zero_index),
                            instance_curr!(meta, input),
                        )),
            ]
        });

        WasmInputHelperTableConfig {
            lookup,
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
        meta.shuffle(key, |meta| {
            let mut exprs = expr(meta);

            let index = exprs.remove(0);
            let value = exprs.remove(0);
            let lookup_value = encode_input_lookup(index, value);

            vec![
                (lookup_value, curr!(meta, self.lookup)), // (exprs.remove(0), fixed_curr!(meta, self.from_zero_index)),
                                                          // (exprs.remove(0), instance_prev!(meta, self.input)),
            ]
        });
    }
}
