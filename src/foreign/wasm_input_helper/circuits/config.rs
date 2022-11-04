use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

use crate::{foreign::ForeignTableConfig, traits::circuits::bit_range_table::BitRangeTable};

use super::WasmInputHelperTableConfig;

impl<F: FieldExt> WasmInputHelperTableConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>, _rtable: &impl BitRangeTable<F>) -> Self {
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
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), self.opcode_expr(meta))]);
    }
}
