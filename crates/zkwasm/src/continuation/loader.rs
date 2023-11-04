use crate::circuits::etable::EVENT_TABLE_ENTRY_ROWS;
use crate::loader::ZkWasmLoader;

use halo2_proofs::arithmetic::MultiMillerLoop;

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
    /// Compute the capability(the length of etable entry) with circuit size K.
    pub fn slice_capability_with_k(&self) -> usize {
        ((1 << self.k as usize) - 200) / EVENT_TABLE_ENTRY_ROWS as usize
    }
}
