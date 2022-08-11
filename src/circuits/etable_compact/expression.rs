use super::*;

impl<F: FieldExt> EventTableCommonConfig<F> {
    pub fn enabled_block(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.shared_bits,
            EventTableBitColumnRotation::Enable as i32
        )
    }

    pub fn op_enabled(&self, meta: &mut VirtualCells<F>, lvl1: i32, lvl2: i32) -> Expression<F> {
        nextn!(meta, self.opcode_bits, lvl1) * nextn!(meta, self.opcode_bits, lvl2)
    }

    pub fn rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::RestMOps as i32
        )
    }

    pub fn next_rest_mops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::RestMOps as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn rest_jops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::RestJOps as i32
        )
    }

    pub fn next_rest_jops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::RestJOps as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::EID as i32
        )
    }

    pub fn next_eid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::EID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn moid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::MOID as i32
        )
    }

    pub fn next_moid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::MOID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn fid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::FID as i32
        )
    }

    pub fn next_fid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::FID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn iid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::IID as i32
        )
    }

    pub fn next_iid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::IID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::MMID as i32
        )
    }

    pub fn next_mmid(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::MMID as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn sp(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::SP as i32
        )
    }

    pub fn next_sp(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.state,
            EventTableCommonRangeColumnRotation::SP as i32 + ETABLE_STEP_SIZE as i32
        )
    }

    pub fn itable_lookup(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux,
            EventTableUnlimitColumnRotation::ITableLookup as i32
        )
    }

    pub fn jtable_lookup(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.aux,
            EventTableUnlimitColumnRotation::JTableLookup as i32
        )
    }

    pub fn mtable_lookup(&self, meta: &mut VirtualCells<F>, i: i32) -> Expression<F> {
        nextn!(
            meta,
            self.aux,
            EventTableUnlimitColumnRotation::MTableLookupStart as i32 + i
        )
    }
}
