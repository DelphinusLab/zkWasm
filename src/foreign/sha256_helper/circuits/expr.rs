use super::{Sha256HelperTableConfig, BLOCK_LINES};
use crate::foreign::sha256_helper::Sha256HelperOp;
use crate::{constant_from, curr, fixed_curr, fixed_next, nextn};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Expression, VirtualCells},
};

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(super) fn arg_to_shift_u32_expr(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> Expression<F> {
        self._arg_to_shift_u32_expr(meta, index, start).0
    }

    pub(super) fn arg_to_shift_u32_expr_with_lowest_u4(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> (Expression<F>, Expression<F>) {
        (
            self.arg_to_shift_u32_expr(meta, index, start),
            nextn!(meta, self.args[index].0, start),
        )
    }

    pub(super) fn arg_to_rotate_u32_expr(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> Expression<F> {
        let (mut acc, mut shift_acc) = self._arg_to_shift_u32_expr(meta, index, start);

        for i in 0..start {
            (acc, shift_acc) = self.shift(meta, index, i, &mut shift_acc, acc);
        }

        acc
    }

    pub(super) fn arg_to_rotate_u32_expr_with_lowest_u4(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> (Expression<F>, Expression<F>) {
        (
            self.arg_to_rotate_u32_expr(meta, index, start),
            nextn!(meta, self.args[index].0, start),
        )
    }

    pub(super) fn is_block_enabled_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        // The first line of op_bit column inside a block is enable bit.
        curr!(meta, self.op_bit.0)
    }

    pub(super) fn opcode_expr(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        curr!(meta, self.aux.0)
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

    pub(self) fn _arg_to_shift_u32_expr(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        start: i32,
    ) -> (Expression<F>, i32) {
        assert!(start < BLOCK_LINES as i32);
        let mut shift_acc: i32 = 0;
        let mut acc = nextn!(meta, self.args[index].0, start);

        for i in start + 1..8 as i32 {
            (acc, shift_acc) = self.shift(meta, index, i, &mut shift_acc, acc);
        }

        (acc, shift_acc)
    }

    pub(self) fn shift(
        &self,
        meta: &mut VirtualCells<'_, F>,
        index: usize,
        i: i32,
        shift_acc: &mut i32,
        acc: Expression<F>,
    ) -> (Expression<F>, i32) {
        *shift_acc += 4;
        (
            acc + nextn!(meta, self.args[index].0, i) * constant_from!(1u64 << *shift_acc),
            *shift_acc,
        )
    }
}
