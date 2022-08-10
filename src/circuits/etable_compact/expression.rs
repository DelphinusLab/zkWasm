use super::*;

impl<F: FieldExt> EventTableCommonConfig<F> {
    fn rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::RestMOps as i32
        )
    }

    fn next_rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::RestMOps as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn rest_jops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::RestJOps as i32
        )
    }

    fn next_rest_jops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::RestJOps as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::EID as i32
        )
    }

    fn next_eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::EID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn moid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::MOID as i32
        )
    }

    fn next_moid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::MOID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn fid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::FID as i32
        )
    }

    fn next_fid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::FID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn iid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::IID as i32
        )
    }

    fn next_iid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::IID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::MMID as i32
        )
    }

    fn next_mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::MMID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    fn sp(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::SP as i32
        )
    }

    fn next_sp(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux_in_common,
            EventTableCommonRangeColumnRotation::SP as i32 + ETABLE_STEP_SIZE as i32
        )
    }
}
