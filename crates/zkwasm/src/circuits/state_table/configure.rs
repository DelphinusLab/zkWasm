use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;

use super::StateTableConfig;
#[cfg(feature = "continuation")]
use super::STATE_COL_NAME;

impl<F: FieldExt> StateTableConfig<F> {
    #[cfg(not(feature = "continuation"))]
    pub(crate) fn configure(_meta: &mut ConstraintSystem<F>) -> Self {
        Self { _mark: PhantomData }
    }

    #[cfg(feature = "continuation")]
    pub(crate) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let col = meta.named_advice_column(STATE_COL_NAME.to_owned());
        meta.enable_equality(col);
        Self {
            col,
            _mark: PhantomData,
        }
    }
}
