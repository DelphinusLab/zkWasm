use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use std::marker::PhantomData;

use super::utils::Context;

mod poseidon;

pub(crate) struct CheckSumConfig<F: FieldExt> {
    _mark: PhantomData<F>,
}

pub(crate) struct CheckSumChip<F: FieldExt> {
    config: CheckSumConfig<F>,
}

impl<F: FieldExt> CheckSumConfig<F> {
    pub(crate) fn configure(_meta: &mut ConstraintSystem<F>) -> Self {
        Self { _mark: PhantomData }
    }
}

impl<F: FieldExt> CheckSumChip<F> {
    pub(crate) fn assign(&self, _ctx: &mut Context<'_, F>) {}
}
