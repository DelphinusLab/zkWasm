use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::Instance;

pub mod assign;
pub mod config;

pub const WASM_INPUT_FOREIGN_TABLE_KEY: &'static str = "wasm-input-helper-table";
const K: usize = 15;
const ENABLE_LINES: usize = 1 << (K - 1);

#[derive(Clone)]
pub struct WasmInputHelperTableConfig<F: FieldExt> {
    enable: Column<Fixed>,
    index: Column<Fixed>,
    input: Column<Instance>,
    _mark: PhantomData<F>,
}
