use halo2_proofs::{arithmetic::FieldExt, plonk::Error};

use crate::circuits::{mtable_v2::MemoryTableChip, utils::table_entry::MemoryWritingTable};

impl<F: FieldExt> MemoryTableChip<F> {
    pub(crate) fn assign(self, mtable: &MemoryWritingTable) -> Result<(), Error> {
        todo!();

        Ok(())
    }
}
