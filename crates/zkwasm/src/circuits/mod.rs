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

#[cfg(feature = "continuation")]
#[path = "./post_image_table/continuation.rs"]
pub mod post_image_table;

#[cfg(not(feature = "continuation"))]
#[path = "./post_image_table/trivial.rs"]
pub mod post_image_table;

pub mod config;
pub mod image_table;
pub mod jtable;
pub mod rtable;
pub mod test_circuit;
pub mod utils;

pub type CompilationTable = specs::CompilationTable;
pub type ExecutionTable = specs::ExecutionTable;

#[derive(Clone)]
pub struct ZkWasmCircuit<F: FieldExt> {
    pub tables: Tables,
    pub slice_capability: Option<usize>,
    _data: PhantomData<F>,
}

impl<F: FieldExt> ZkWasmCircuit<F> {
    pub fn new(tables: Tables, slice_capability: Option<usize>) -> Self {
        ZkWasmCircuit {
            tables,
            slice_capability,
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
}

impl ZkWasmCircuitBuilder {
    pub fn build_circuit<F: FieldExt>(self, slice_capability: Option<usize>) -> ZkWasmCircuit<F> {
        #[cfg(feature = "continuation")]
        assert!(slice_capability.is_some());

        #[cfg(not(feature = "continuation"))]
        assert!(slice_capability.is_none());

        ZkWasmCircuit::new(self.tables, slice_capability)
    }
}
