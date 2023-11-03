use halo2_proofs::arithmetic::FieldExt;

use crate::circuits::TestCircuit;
use crate::circuits::ZkWasmCircuitBuilder;

impl ZkWasmCircuitBuilder {
    pub fn build_slice_circuit<F: FieldExt>(self, slice_capability: usize) -> TestCircuit<F> {
        TestCircuit::new_slice(self.tables, slice_capability)
    }
}
