use super::JumpTableConfig;
use crate::curr;
use crate::fixed_curr;
use crate::next;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::frame_table::encode_frame_table_entry;

impl<F: FieldExt> JumpTableConfig<F> {
    pub(super) fn enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.enable)
    }

    pub(super) fn rest_call_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.call_ops)
    }

    pub(super) fn next_rest_call_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        next!(meta, self.call_ops)
    }

    pub(super) fn rest_return_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.return_ops)
    }

    pub(super) fn next_rest_return_ops(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        next!(meta, self.return_ops)
    }

    pub(super) fn returned(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.returned)
    }

    pub(super) fn encode(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        curr!(meta, self.encode)
    }

    pub(super) fn inherited_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        fixed_curr!(meta, self.inherited)
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
