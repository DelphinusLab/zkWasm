use std::marker::PhantomData;

use ark_std::One;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use num_bigint::BigUint;
use specs::encode::FromBn;

pub mod assign;
pub mod config;

pub const WASM_INPUT_FOREIGN_TABLE_KEY: &'static str = "wasm-input-helper-table";

#[derive(Clone)]
pub struct WasmInputHelperTableConfig<F: FieldExt> {
    lookup: Column<Advice>,
    _mark: PhantomData<F>,
}

pub(self) fn encode_input_lookup<T: FromBn>(index: T, value: T) -> T {
    index * T::from_bn(&(BigUint::one() << 64)) + value
}
