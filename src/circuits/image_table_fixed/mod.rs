use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::TableColumn;
use std::marker::PhantomData;

mod assign;
mod configure;

#[derive(Clone)]
pub struct ImageTableConfig<F: FieldExt> {
    col: TableColumn,
    _mark: PhantomData<F>,
}

pub struct ImageTableChip<F: FieldExt> {
    config: ImageTableConfig<F>,
}

impl<F: FieldExt> ImageTableChip<F> {
    pub fn new(config: ImageTableConfig<F>) -> Self {
        ImageTableChip { config }
    }
}
