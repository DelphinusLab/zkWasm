use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Expression, VirtualCells},
};

use crate::{fixed_curr, instance_curr};

use super::{InputTableEncode, WasmInputHelperTableConfig};

impl<F: FieldExt> WasmInputHelperTableConfig<F> {
    pub(super) fn opcode_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        fixed_curr!(meta, self.enable)
            * InputTableEncode::encode_for_lookup(
                fixed_curr!(meta, self.index),
                instance_curr!(meta, self.input),
            )
    }
}
