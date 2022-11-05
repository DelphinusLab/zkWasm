use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Column, Fixed, Instance},
};
use num_bigint::BigUint;
use specs::encode::FromBn;

pub mod assign;
pub mod config;
pub mod expr;

pub const WASM_INPUT_FOREIGN_TABLE_KEY: &'static str = "wasm-input-helper-table";
const K: usize = 15;
pub const ENABLE_LINES: usize = 1 << (K - 1);

pub struct InputTableEncode {}

impl InputTableEncode {
    pub(super) fn encode_for_lookup<T: FromBn>(index: T, input: T) -> T {
        T::from_bn(&(BigUint::from(1u64) << 128))
            + index * T::from_bn(&(BigUint::from(1u64) << 64))
            + input
    }
}

#[derive(Clone)]
pub struct WasmInputHelperTableConfig<F: FieldExt> {
    enable: Column<Fixed>,
    index: Column<Fixed>,
    input: Column<Instance>,
    _mark: PhantomData<F>,
}
