use crate::circuits::utils::Context;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::Tables;
use std::marker::PhantomData;

pub(crate) mod cell;
pub(crate) mod etable;

mod bit_table;
mod external_host_call_table;
mod mtable;
mod traits;

pub mod config;
pub mod image_table;
pub mod jtable;
pub mod rtable;
pub mod test_circuit;
pub mod utils;

#[derive(Default, Clone)]
pub struct TestCircuit<F: FieldExt> {
    pub tables: Tables,
    _data: PhantomData<F>,
}

impl<F: FieldExt> TestCircuit<F> {
    pub fn new(tables: Tables) -> Self {
        TestCircuit {
            tables,
            _data: PhantomData,
        }
    }
}

trait Encode {
    fn encode(&self) -> BigUint;
}

pub(self) trait Lookup<F: FieldExt> {
    fn encode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;

    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: impl FnOnce(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), self.encode(meta))]);
    }
}

pub struct ZkWasmCircuitBuilder {
    pub tables: Tables,
    pub public_inputs_and_outputs: Vec<u64>,
}

impl ZkWasmCircuitBuilder {
    pub fn build_circuit<F: FieldExt>(&self) -> TestCircuit<F> {
        TestCircuit::new(self.tables.clone())
    }
}
