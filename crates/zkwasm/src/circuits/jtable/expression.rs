use super::JumpTableConfig;
use crate::circuits::jtable::FrameTableValueOffset;
use crate::fixed_curr;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::frame_table::encode_frame_table_entry;

impl<F: FieldExt> JumpTableConfig<F> {
    pub(super) fn enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.value, FrameTableValueOffset::Enable as i32)
    }

    pub(super) fn rest_call_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.value, FrameTableValueOffset::CallOps as i32)
    }

    pub(super) fn next_rest_call_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.value,
            FrameTableValueOffset::CallOps as i32 + FrameTableValueOffset::Max as i32
        )
    }

    pub(super) fn rest_return_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.value, FrameTableValueOffset::ReturnOps as i32)
    }

    pub(super) fn next_rest_return_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(
            meta,
            self.value,
            FrameTableValueOffset::ReturnOps as i32 + FrameTableValueOffset::Max as i32
        )
    }

    pub(super) fn returned(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.value, FrameTableValueOffset::Returned as i32)
    }

    pub(super) fn encode(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        nextn!(meta, self.value, FrameTableValueOffset::Encode as i32)
    }

    pub(super) fn inherited_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_curr!(meta, self.inherited)
    }

    pub(super) fn sel(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_curr!(meta, self.sel)
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
