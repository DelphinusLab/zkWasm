use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

use crate::{
    constant_from, curr, fixed_curr, foreign::ForeignTableConfig, instance_curr,
    traits::circuits::bit_range_table::BitRangeTable,
};

use super::WasmInputHelperTableConfig;

impl<F: FieldExt> WasmInputHelperTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, rtable: &impl BitRangeTable<F>) -> Self {
        let enable = meta.fixed_column();
        let index = meta.fixed_column();
        let input = meta.instance_column();
        let input_u8 = [(); 8].map(|_| meta.advice_column());

        // constaint input to u64
        for i in input_u8 {
            rtable.configure_in_u8_range(meta, "wasm input u8 cell", |meta| curr!(meta, i));
        }

        meta.create_gate("input u8 equals input", |meta| {
            let mut acc = constant_from!(0);

            for i in 0..8 {
                acc = acc + curr!(meta, input_u8[i]) * constant_from!(1u64 << (8 * i));
            }

            vec![fixed_curr!(meta, enable) * (acc - instance_curr!(meta, input))]
        });

        WasmInputHelperTableConfig {
            enable,
            index,
            input,
            input_u8,
            _mark: std::marker::PhantomData,
        }
    }
}

impl<F: FieldExt> ForeignTableConfig<F> for WasmInputHelperTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), self.opcode_expr(meta))]);
    }
}
