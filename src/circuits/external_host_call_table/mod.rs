use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Advice, Column, Fixed},
};
use std::marker::PhantomData;

use crate::circuits::config::{
    zkwasm_k, ZKWASM_FOREIGN_CALL_TABLE_RATIO, ZKWASM_TABLE_DENOMINATOR,
};

use super::traits::TableSize;

mod assign;
mod configure;

#[derive(Clone)]
pub struct ExternalHostCallTableConfig<F: FieldExt> {
    idx: Column<Fixed>,
    op: Column<Advice>,
    arg: Column<Advice>,
    is_ret: Column<Advice>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> TableSize for ExternalHostCallTableConfig<F> {
    fn max_available_size(&self) -> usize {
        assert!(*ZKWASM_FOREIGN_CALL_TABLE_RATIO < *ZKWASM_TABLE_DENOMINATOR);

        ((1 << zkwasm_k()) / *ZKWASM_TABLE_DENOMINATOR * *ZKWASM_FOREIGN_CALL_TABLE_RATIO) as usize
    }
}

pub struct ExternalHostCallChip<F: FieldExt> {
    config: ExternalHostCallTableConfig<F>,
}

impl<F: FieldExt> ExternalHostCallChip<F> {
    pub fn new(config: ExternalHostCallTableConfig<F>) -> Self {
        Self { config }
    }
}
