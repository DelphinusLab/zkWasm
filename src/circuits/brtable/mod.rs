use halo2_proofs::{arithmetic::FieldExt, plonk::TableColumn};
use std::marker::PhantomData;

mod assign;
mod configure;

#[derive(Clone)]
pub struct BrTableConfig<F: FieldExt> {
    pub(self) col: TableColumn,
    _mark: PhantomData<F>,
}

pub struct BrTableChip<F: FieldExt> {
    config: BrTableConfig<F>,
}

impl<F: FieldExt> BrTableChip<F> {
    pub fn new(config: BrTableConfig<F>) -> Self {
        BrTableChip { config }
    }
}
