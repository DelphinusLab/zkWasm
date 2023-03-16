use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

use crate::circuits::Lookup;
use crate::curr;
use crate::fixed_curr;

use super::BitTableConfig;

impl<F: FieldExt> Lookup<F> for BitTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| {
            vec![(
                expr(meta),
                curr!(meta, self.value) * fixed_curr!(meta, self.step_sel),
            )]
        });
    }

    fn encode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        curr!(meta, self.value) * fixed_curr!(meta, self.step_sel)
    }
}
