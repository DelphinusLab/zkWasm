use super::JtableOffset;
use super::JumpTableConfig;
use crate::fixed_curr;
use crate::fixed_nextn;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::frame_table::encode_frame_table_entry;

impl<F: FieldExt> JumpTableConfig<F> {
    pub(super) fn enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.data, JtableOffset::JtableOffsetEnable as i32)
    }

    pub(super) fn rest(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.data, JtableOffset::JtableOffsetRest as i32)
    }

    pub(super) fn next_rest(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.data,
            JtableOffset::JtableOffsetRest as i32 + JtableOffset::JtableOffsetMax as i32
        )
    }

    pub(super) fn entry(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.data, JtableOffset::JtableOffsetEntry as i32)
    }

    pub(super) fn next_entry(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.data,
            JtableOffset::JtableOffsetEntry as i32 + JtableOffset::JtableOffsetMax as i32
        )
    }

    pub(super) fn static_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_curr!(meta, self.static_bit)
    }

    pub(super) fn next_static_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_nextn!(meta, self.static_bit, JtableOffset::JtableOffsetMax as i32)
    }
}

pub(crate) trait JtableLookupEntryEncode<F> {
    fn encode_lookup(
        current_last_jump_eid: Expression<F>,
        next_last_jump_eid: Expression<F>,
        callee_fid: Expression<F>,
        next_fid: Expression<F>,
        next_iid: Expression<F>,
    ) -> Expression<F>;
}

impl<F: FieldExt> JtableLookupEntryEncode<F> for JumpTableConfig<F> {
    fn encode_lookup(
        current_last_jump_eid: Expression<F>,
        next_last_jump_eid: Expression<F>,
        callee_fid: Expression<F>,
        next_fid: Expression<F>,
        next_iid: Expression<F>,
    ) -> Expression<F> {
        encode_frame_table_entry(
            current_last_jump_eid,
            next_last_jump_eid,
            callee_fid,
            next_fid,
            next_iid,
        )
    }
}
