use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Fixed;
use halo2_proofs::plonk::Instance;

pub mod config;

pub const WASM_INPUT_FOREIGN_TABLE_KEY: &'static str = "wasm-input-helper-table";

#[derive(Clone)]
pub struct WasmInputHelperTableConfig<F: FieldExt> {
    from_zero_index: Column<Fixed>,
    input: Column<Instance>,
    _mark: PhantomData<F>,
}
