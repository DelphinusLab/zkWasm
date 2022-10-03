use super::{Sha2HelperConfig, BLOCK_LINES};
use crate::foreign::sha256_helper::Sha256HelperOp;
use crate::{constant_from, curr, fixed_curr, fixed_next, nextn};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Expression, VirtualCells},
};

impl<F: FieldExt> Sha2HelperConfig<F> {
    pub(super) fn arg_to_u32_expr(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> Expression<F> {
        assert!(start < BLOCK_LINES as i32);
        let mut shift_acc = 1;
        let mut acc = nextn!(meta, self.args[index].0, start);

        for i in start + 1..BLOCK_LINES as i32 {
            shift_acc += 4;
            acc = acc + nextn!(meta, self.args[index].0, i) * constant_from!(1u64 << shift_acc);
        }

        acc
    }

    pub(super) fn opcode_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        fixed_curr!(meta, self.block_first_line_sel) * curr!(meta, self.aux.0)
    }

    pub(super) fn is_not_block_end_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        fixed_next!(meta, self.sel)
            * (constant_from!(1) - fixed_next!(meta, self.block_first_line_sel))
    }

    pub(super) fn is_op_enabled_expr(
        &self,
        meta: &mut VirtualCells<F>,
        index: Sha256HelperOp,
    ) -> Expression<F> {
        fixed_curr!(meta, self.block_first_line_sel) * nextn!(meta, self.op_bit.0, index as i32)
    }
}
