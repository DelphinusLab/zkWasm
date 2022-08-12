use super::{JtableOffset, JumpTableConfig};
use crate::{circuits::utils::bn_to_field, constant, nextn};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Expression, VirtualCells},
};
use num_bigint::BigUint;

const EID_SHIFT: usize = 64;
const LAST_JUMP_EID_SHIFT: usize = 48;
const MOID_SHIFT: usize = 32;
const FID_SHIFT: usize = 16;

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
}

pub(crate) trait JtableLookupEntryEncode<F> {
    fn encode_lookup(
        current_last_jump_eid: Expression<F>,
        next_last_jump_eid: Expression<F>,
        next_moid: Expression<F>,
        next_fid: Expression<F>,
        next_iid: Expression<F>,
    ) -> Expression<F>;
}

impl<F: FieldExt> JtableLookupEntryEncode<F> for JumpTableConfig<F> {
    fn encode_lookup(
        current_last_jump_eid: Expression<F>,
        next_last_jump_eid: Expression<F>,
        next_moid: Expression<F>,
        next_fid: Expression<F>,
        next_iid: Expression<F>,
    ) -> Expression<F> {
        let one = BigUint::from(1u64);

        current_last_jump_eid * constant!(bn_to_field(&(&one << EID_SHIFT)))
            + next_last_jump_eid * constant!(bn_to_field(&(&one << LAST_JUMP_EID_SHIFT)))
            + next_moid * constant!(bn_to_field(&(&one << MOID_SHIFT)))
            + next_fid * constant!(bn_to_field(&(&one << FID_SHIFT)))
            + next_iid
    }
}
