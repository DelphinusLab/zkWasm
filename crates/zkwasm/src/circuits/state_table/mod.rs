use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use std::marker::PhantomData;

pub(crate) mod assign;
pub(crate) mod configure;

pub const STATE_COL_NAME: &str = "state_col";

#[derive(Clone)]
pub struct StateTableConfig<F: FieldExt> {
    #[cfg(feature = "continuation")]
    col: Column<Advice>,
    _mark: PhantomData<F>,
}

#[derive(Clone)]
pub struct StateTableChip<F: FieldExt> {
    config: StateTableConfig<F>,
}

impl<F: FieldExt> StateTableChip<F> {
    pub fn new(config: StateTableConfig<F>) -> Self {
        StateTableChip { config }
    }
}
