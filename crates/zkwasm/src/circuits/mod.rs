use crate::circuits::utils::Context;
use crate::error::BuildingCircuitError;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::dev::MockProver;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use num_bigint::BigUint;
use specs::slice::Slice;
use std::marker::PhantomData;

use self::etable::EVENT_TABLE_ENTRY_ROWS;
use self::image_table::compute_maximal_pages;
use self::zkwasm_circuit::RESERVE_ROWS;

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

pub mod image_table;
pub mod jtable;
pub mod rtable;
pub mod utils;
pub mod zkwasm_circuit;

pub const MIN_K: u32 = 18;

pub type CompilationTable = specs::CompilationTable;
pub type ExecutionTable<B> = specs::ExecutionTable<B>;

pub(crate) fn compute_slice_capability(k: u32) -> u32 {
    ((1 << k) - RESERVE_ROWS as u32 - 1024) / EVENT_TABLE_ENTRY_ROWS as u32
}

pub struct OngoingCircuit<F: FieldExt> {
    pub k: u32,
    pub slice: Slice,
    _data: PhantomData<F>,
}

impl<F: FieldExt> OngoingCircuit<F> {
    pub fn new(k: u32, slice: Slice) -> Result<Self, BuildingCircuitError> {
        {
            // entries is empty when called by without_witness
            let allocated_memory_pages = slice
                .etable
                .entries()
                .last()
                .map(|entry| entry.allocated_memory_pages);
            let maximal_pages = compute_maximal_pages(k);
            if let Some(allocated_memory_pages) = allocated_memory_pages {
                if allocated_memory_pages > maximal_pages {
                    return Err(BuildingCircuitError::PagesExceedLimit(
                        allocated_memory_pages,
                        maximal_pages,
                        k,
                    ));
                }
            }
        }

        {
            let etable_entires = slice.etable.entries().len() as u32;
            let etable_capacity = compute_slice_capability(k);

            if etable_entires > etable_capacity {
                return Err(BuildingCircuitError::EtableEntriesExceedLimit(
                    etable_entires,
                    etable_capacity,
                    k,
                ));
            }
        }

        Ok(OngoingCircuit {
            k,
            slice,
            _data: PhantomData,
        })
    }
}

pub struct LastSliceCircuit<F: FieldExt> {
    pub k: u32,
    pub slice: Slice,
    _data: PhantomData<F>,
}

impl<F: FieldExt> LastSliceCircuit<F> {
    pub fn new(k: u32, slice: Slice) -> Result<Self, BuildingCircuitError> {
        {
            // entries is empty when called by without_witness
            let allocated_memory_pages = slice
                .etable
                .entries()
                .last()
                .map(|entry| entry.allocated_memory_pages);
            let maximal_pages = compute_maximal_pages(k);
            if let Some(allocated_memory_pages) = allocated_memory_pages {
                if allocated_memory_pages > maximal_pages {
                    return Err(BuildingCircuitError::PagesExceedLimit(
                        allocated_memory_pages,
                        maximal_pages,
                        k,
                    ));
                }
            }
        }

        {
            let etable_entires = slice.etable.entries().len() as u32;
            let etable_capacity = compute_slice_capability(k);

            if etable_entires > etable_capacity {
                return Err(BuildingCircuitError::EtableEntriesExceedLimit(
                    etable_entires,
                    etable_capacity,
                    k,
                ));
            }
        }

        Ok(LastSliceCircuit {
            k,
            slice,
            _data: PhantomData,
        })
    }
}
pub enum ZkWasmCircuit<F: FieldExt> {
    Ongoing(OngoingCircuit<F>),
    LastSliceCircuit(LastSliceCircuit<F>),
}

impl<F: FieldExt> ZkWasmCircuit<F> {
    pub fn new(k: u32, slice: Slice) -> Result<Self, BuildingCircuitError> {
        if slice.is_last_slice {
            Ok(ZkWasmCircuit::LastSliceCircuit(LastSliceCircuit::new(
                k, slice,
            )?))
        } else {
            Ok(ZkWasmCircuit::Ongoing(OngoingCircuit::new(k, slice)?))
        }
    }

    pub fn mock_test(&self, instances: Vec<F>) -> anyhow::Result<()> {
        match self {
            ZkWasmCircuit::Ongoing(circuit) => {
                let prover = MockProver::run(circuit.k, circuit, vec![instances])?;
                assert_eq!(prover.verify(), Ok(()));
            }
            ZkWasmCircuit::LastSliceCircuit(circuit) => {
                let prover = MockProver::run(circuit.k, circuit, vec![instances])?;
                assert_eq!(prover.verify(), Ok(()));
            }
        }

        Ok(())
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
