use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Expression, VirtualCells},
};

use crate::nextn;

use super::{
    JumpTableConfig, JTABLE_OFFSET_AUX, JTABLE_OFFSET_ENTRY, JTABLE_OFFSET_REST, JTABLE_STEP_SIZE,
};

impl<F: FieldExt> JumpTableConfig<F> {
    pub(super) fn rest(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.data, JTABLE_OFFSET_REST as i32)
    }

    pub(super) fn next_rest(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.data,
            JTABLE_OFFSET_REST as i32 + JTABLE_STEP_SIZE
        )
    }

    pub(super) fn entry(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.data, JTABLE_OFFSET_ENTRY as i32)
    }

    pub(super) fn aux(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.data, JTABLE_OFFSET_AUX as i32)
    }
}
