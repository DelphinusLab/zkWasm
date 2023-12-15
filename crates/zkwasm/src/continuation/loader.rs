use halo2_proofs::arithmetic::MultiMillerLoop;
use wasmi::RuntimeValue;

use crate::circuits::etable::EVENT_TABLE_ENTRY_ROWS;
use crate::loader::ZkWasmLoader;
use crate::runtime::ExecutionResult;

use super::slice::Slices;

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
    pub(crate) fn compute_slice_capability(&self) -> usize {
        ((1 << self.k) - 200) / EVENT_TABLE_ENTRY_ROWS as usize
    }

    pub fn slice(&self, execution_result: ExecutionResult<RuntimeValue>) -> Slices {
        Slices::new(execution_result.tables, self.compute_slice_capability())
    }
}
