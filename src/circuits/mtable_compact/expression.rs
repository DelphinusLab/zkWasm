use super::configure::STEP_SIZE;
use super::lookup::MtableLookupEntryEncode;
use super::*;
use crate::circuits::utils::bn_to_field;
use crate::constant_from;
use crate::constant_from_bn;
use crate::curr;
use crate::fixed_curr;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::mtable::LocationType;

pub(super) enum RotationIndex {
    LTYPE = 0,
    MMID,
    OFFSET,
    EID,
    EMID,
    MAX,
}

pub(super) enum RotationAux {
    ConstantOne = 0,
    SameLtype,
    SameMmid,
    SameOffset,
    SameEid,
    Atype,
    RestMops,
}

pub(crate) const ROTATION_VTYPE_GE_TWO_BYTES: i32 = 1;
pub(crate) const ROTATION_VTYPE_GE_FOUR_BYTES: i32 = 2;
pub(crate) const ROTATION_VTYPE_GE_EIGHT_BYTES: i32 = 3;
pub(crate) const ROTATION_VTYPE_SIGN: i32 = 4;

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(super) fn is_enabled_block(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.bit)
            * fixed_curr!(meta, self.sel)
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
        nextn!(meta, self.index.same, RotationIndex::LTYPE as i32)
    }

    pub(super) fn same_mmid_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationIndex::MMID as i32)
    }

    pub(super) fn same_offset_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationIndex::OFFSET as i32)
    }

    pub(super) fn same_eid_single(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.same, RotationIndex::EID as i32)
    }

    pub(super) fn same_ltype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        self.same_ltype_single(meta)
    }

    pub(super) fn same_mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationAux::SameMmid as i32)
    }

    pub(super) fn same_offset(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationAux::SameOffset as i32)
    }

    pub(super) fn same_eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationAux::SameEid as i32)
    }

    pub(super) fn ltype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationIndex::LTYPE as i32)
    }

    pub(super) fn mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationIndex::MMID as i32)
    }

    pub(super) fn offset(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationIndex::OFFSET as i32)
    }

    pub(super) fn eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationIndex::EID as i32)
    }

    pub(super) fn emid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.index.data, RotationIndex::EMID as i32)
    }

    pub(super) fn is_heap(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        assert_eq!(LocationType::Heap as u64, 0u64);
        assert_eq!(LocationType::Stack as u64, 1u64);
        constant_from!(1) - self.ltype(meta)
    }

    pub(super) fn atype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationAux::Atype as i32)
    }

    pub(super) fn prev_atype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationAux::Atype as i32 - STEP_SIZE)
    }

    pub(super) fn vtype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        (self.ge_two_bytes(meta) + self.ge_four_bytes(meta) + self.ge_eight_bytes(meta))
            * constant_from!(2)
            + self.sign(meta)
            + constant_from!(1)
    }

    pub(super) fn prev_vtype(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        (self.prev_ge_two_bytes(meta)
            + self.prev_ge_four_bytes(meta)
            + self.prev_ge_eight_bytes(meta))
            * constant_from!(2)
            + self.sign(meta)
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
        nextn!(meta, self.aux, RotationAux::RestMops as i32)
    }

    pub(super) fn prev_rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.aux, RotationAux::RestMops as i32 - STEP_SIZE)
    }

    pub(super) fn byte(&self, meta: &mut VirtualCells<F>, index: i32) -> Expression<F> {
        nextn!(meta, self.bytes, index)
    }

    pub(super) fn ge_two_bytes(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_GE_TWO_BYTES)
    }

    pub(super) fn ge_four_bytes(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_GE_FOUR_BYTES)
    }

    pub(super) fn ge_eight_bytes(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_GE_EIGHT_BYTES)
    }

    pub(super) fn sign(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_SIGN)
    }

    pub(super) fn prev_ge_two_bytes(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_GE_TWO_BYTES)
    }

    pub(super) fn prev_ge_four_bytes(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_GE_FOUR_BYTES)
    }

    pub(super) fn prev_ge_eight_bytes(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_GE_EIGHT_BYTES)
    }

    pub(super) fn prev_sign(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.bit, ROTATION_VTYPE_SIGN)
    }
}

impl<F: FieldExt> MtableLookupEntryEncode<Expression<F>> for MemoryTableConfig<F> {
    fn bn_to_t(v: &BigUint) -> Expression<F> {
        constant_from_bn!(v)
    }
}
