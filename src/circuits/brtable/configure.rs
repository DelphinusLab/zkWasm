use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::TableColumn;
use halo2_proofs::plonk::VirtualCells;
use std::marker::PhantomData;

use super::BrTableConfig;
use crate::circuits::traits::ConfigureLookupTable;

impl<F: FieldExt> BrTableConfig<F> {
    pub(in crate::circuits) fn configure(col: TableColumn) -> Self {
        Self {
            col,
            _mark: PhantomData,
        }
    }
}

impl<F: FieldExt> ConfigureLookupTable<F> for BrTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup(key, |meta| vec![(expr(meta), self.col)]);
    }
}
