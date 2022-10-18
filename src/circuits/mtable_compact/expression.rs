use super::configure::STEP_SIZE;
use super::*;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::mtable::LocationType;

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(super) fn is_enabled_block(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.bit)
            * fixed_curr!(meta, self.block_first_line_sel)
    }

    pub(super) fn is_enabled_following_block(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.bit)
            * fixed_curr!(meta, self.block_first_line_sel)
            * fixed_curr!(meta, self.following_block_sel)
    }

    pub(super) fn is_enabled_line(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_curr!(meta, self.sel)
    }

    pub(super) fn is_enabled_following_line(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_curr!(meta, self.following_block_sel)
    }

    pub(super) fn same_ltype_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationOfIndexColumn::LTYPE as i32)
    }

    pub(super) fn same_mmid_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationOfIndexColumn::MMID as i32)
    }

    pub(super) fn same_offset_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationOfIndexColumn::OFFSET as i32)
    }

    pub(super) fn same_eid_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationOfIndexColumn::EID as i32)
    }

    pub(super) fn same_ltype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        self.same_ltype_single(meta)
    }

    pub(super) fn same_mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationOfAuxColumn::SameMmid as i32)
    }

    pub(super) fn same_offset(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationOfAuxColumn::SameOffset as i32)
    }

    pub(super) fn same_eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationOfAuxColumn::SameEid as i32)
    }

    pub(super) fn ltype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationOfIndexColumn::LTYPE as i32)
    }

    pub(super) fn mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationOfIndexColumn::MMID as i32)
    }

    pub(super) fn offset(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationOfIndexColumn::OFFSET as i32)
    }

    pub(super) fn eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationOfIndexColumn::EID as i32)
    }

    pub(super) fn emid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationOfIndexColumn::EMID as i32)
    }

    pub(super) fn atype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationOfAuxColumn::Atype as i32)
    }

    pub(super) fn prev_atype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux,
            RotationOfAuxColumn::Atype as i32 - STEP_SIZE
        )
    }

    fn vtype_composer(&self, is_i64: Expression<F>) -> Expression<F> {
        is_i64 + constant_from!(1)
    }

    pub(super) fn vtype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        self.vtype_composer(self.is_i64(meta))
    }

    pub(super) fn prev_vtype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        self.vtype_composer(self.prev_is_i64(meta))
    }

    pub(super) fn value(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let mut acc = self.byte(meta, 0);
        let mut base = 1u64;
        for i in 1..8 {
            base <<= 8;
            acc = acc + constant_from!(base) * self.byte(meta, i);
        }
        acc
    }

    pub(super) fn prev_value(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let mut acc = self.byte(meta, -STEP_SIZE);
        let mut base = 1u64;
        for i in 1..8 {
            base <<= 8;
            acc = acc + constant_from!(base) * self.byte(meta, i - STEP_SIZE);
        }
        acc
    }

    pub(super) fn rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationOfAuxColumn::RestMops as i32)
    }

    pub(super) fn prev_rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux,
            RotationOfAuxColumn::RestMops as i32 - STEP_SIZE
        )
    }

    pub(super) fn byte(&self, meta: &mut VirtualCells<F>, index: i32) -> Expression<F> {
        nextn!(meta, self.bytes, index)
    }

    pub(super) fn is_i64(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, RotationOfBitColumn::Is64Bit as i32)
    }

    pub(super) fn prev_is_i64(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, RotationOfBitColumn::Is64Bit as i32 - STEP_SIZE)
    }

    pub(super) fn is_mutable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, RotationOfBitColumn::IsMutable as i32)
    }

    pub(super) fn prev_is_mutable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, RotationOfBitColumn::IsMutable as i32 - STEP_SIZE)
    }

    pub(super) fn is_stack(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, RotationOfBitColumn::IsStack as i32)
    }

    pub(super) fn imtable_selector(&self, meta: &mut VirtualCells<F>, i: u32) -> Expression<F> {
        assert!((i as u32 + RotationOfBitColumn::IMTableSelectorStart as u32) < STEP_SIZE as u32);
        nextn!(meta, self.bit, RotationOfBitColumn::IMTableSelectorStart as i32 + i as i32)
    }
}
