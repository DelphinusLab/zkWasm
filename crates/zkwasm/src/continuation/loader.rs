use crate::loader::ZkWasmLoader;

use halo2_proofs::arithmetic::MultiMillerLoop;

impl<E: MultiMillerLoop> ZkWasmLoader<E> {
    /// Compute the capability(the length of etable entry) with circuit size K.
    pub fn slice_capability_with_k(&self) -> usize {
        todo!("compute maximal etable entries with self.k");
    }
}
