use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;

pub mod assign;
pub mod config;

pub const CONTEXT_FOREIGN_TABLE_KEY: &str = "wasm-context-helper-table";

#[derive(Clone)]
pub struct ContextContHelperTableConfig<F: FieldExt> {
    from_zero_index: Column<Fixed>,
    input: Column<Advice>,
    output: Column<Advice>,
    _mark: PhantomData<F>,
}
