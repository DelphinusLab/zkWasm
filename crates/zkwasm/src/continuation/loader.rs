use halo2_proofs::arithmetic::MultiMillerLoop;
use wasmi::RuntimeValue;

use crate::loader::ZkWasmLoader;
use crate::runtime::ExecutionResult;

use super::slice::Slices;

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
    pub fn slice(&self, execution_result: ExecutionResult<RuntimeValue>) -> Slices {
        Slices::new(execution_result.tables.unwrap(), self.compute_slice_capability())
    }
}
