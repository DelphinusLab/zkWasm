use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{constant_from, curr, foreign::sha256_helper::circuits::Sha2HelperEncode};
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};

const OP: Sha256HelperOp = Sha256HelperOp::Maj;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_maj(&self, meta: &mut ConstraintSystem<F>) {
        // (a & b) ^ (a & c) ^ (b & c)
        meta.create_gate("sha256 maj", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let a = self.arg_to_u32_expr(meta, 0, 0);
            let b = self.arg_to_u32_expr(meta, 1, 0);
            let c = self.arg_to_u32_expr(meta, 2, 0);
            let res = self.arg_to_u32_expr(meta, 4, 0);

            vec![
                enable.clone()
                    * (curr!(meta, self.op.0)
                        - constant_from!(OP)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcocde_expr(curr!(meta, self.op.0), vec![&res, &a, &b, &c])),
            ]
        });
    }
}
