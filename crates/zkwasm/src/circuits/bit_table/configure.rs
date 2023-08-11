use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

use crate::fixed_curr;
use crate::prev;

use super::BitTableConfig;

impl<F: FieldExt> BitTableConfig<F> {
    pub(crate) fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(
            &mut VirtualCells<'_, F>,
        ) -> (
            Expression<F>,
            Expression<F>,
            Expression<F>,
            Expression<F>,
            Expression<F>,
        ),
    ) {
        meta.lookup_any(key, |meta| {
            let (sel, op, left, right, result) = expr(meta);

            vec![
                // To prevent `fixed_curr!(meta, self.block_sel) * expr` which makes the h_poly degree up to 5
                (sel, fixed_curr!(meta, self.block_sel)),
                (op, prev!(meta, self.op)),
                (left, prev!(meta, self.left)),
                (right, prev!(meta, self.right)),
                (result, prev!(meta, self.result)),
            ]
        });
    }
}
