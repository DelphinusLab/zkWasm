use super::*;
use crate::constant;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::next;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::mtable::AccessType;

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(super) fn encode_for_lookup(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        (self.eid.data(meta) * constant!(bn_to_field(&EID_SHIFT))
            + self.emid.data(meta) * constant!(bn_to_field(&EMID_SHIFT))
            + self.mmid.data(meta) * constant!(bn_to_field(&MMID_SHIFT))
            + self.offset.data(meta) * constant!(bn_to_field(&OFFSET_SHIFT))
            + self.ltype.data(meta) * constant!(bn_to_field(&LOC_TYPE_SHIFT))
            + curr!(meta, self.atype) * constant!(bn_to_field(&ACCESS_TYPE_SHIFT))
            + curr!(meta, self.vtype) * constant!(bn_to_field(&VAR_TYPE_SHIFT))
            + curr!(meta, self.value.value))
            * curr!(meta, self.enable)
            * fixed_curr!(meta, self.sel)
    }

    pub(super) fn diff_to_next(
        &self,
        meta: &mut VirtualCells<F>,
        col: Column<Advice>,
    ) -> Expression<F> {
        next!(meta, col) - curr!(meta, col)
    }

    pub(super) fn is_heap(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        assert_eq!(LocationType::Heap as u64, 0u64);
        assert_eq!(LocationType::Stack as u64, 1u64);
        constant_from!(1) - self.ltype.data(meta)
    }

    pub(super) fn is_stack(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        assert_eq!(LocationType::Heap as u64, 0u64);
        assert_eq!(LocationType::Stack as u64, 1u64);
        self.ltype.data(meta)
    }

    pub(super) fn is_enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.enable)
    }

    pub(super) fn is_next_enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        next!(meta, self.enable)
    }

    pub(super) fn is_same_location(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.same_location)
    }

    pub(super) fn is_diff_location(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.same_location) - constant_from!(1)
    }

    pub(super) fn is_next_same_location(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        next!(meta, self.same_location)
    }

    pub(super) fn is_next_read_not_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        (next!(meta, self.atype) - constant_from!(AccessType::Init))
            * (next!(meta, self.atype) - constant_from!(AccessType::Write))
    }

    pub(super) fn is_not_init(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        // lagrange
        let read_f = F::from(AccessType::Read as u64);
        let write_f = F::from(AccessType::Write as u64);
        let init_f = F::from(AccessType::Init as u64);
        let atype = curr!(meta, self.atype);
        (atype.clone() - constant_from!(AccessType::Write))
            * (atype.clone() - constant_from!(AccessType::Init))
            * constant!(((read_f - write_f) * (read_f - init_f)).invert().unwrap())
            + (atype.clone() - constant_from!(AccessType::Read))
                * (atype.clone() - constant_from!(AccessType::Init))
                * constant!(((write_f - read_f) * (write_f - init_f)).invert().unwrap())
    }
}
