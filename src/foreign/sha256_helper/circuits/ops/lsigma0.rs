use super::super::{Sha256HelperOp, Sha2HelperConfig};
use crate::{
    constant_from, curr, foreign::sha256_helper::circuits::Sha2HelperEncode, nextn,
};
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};

const OP: Sha256HelperOp = Sha256HelperOp::LSigma0;

impl<F: FieldExt> Sha2HelperConfig<F> {
    pub(crate) fn configure_lsigma0(&self, meta: &mut ConstraintSystem<F>) {
        // (x >> 2) ^ (x >> 13) ^ (x >> 22)
        meta.create_gate("sha256 lsigma0", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_u32_expr(meta, 0, 0);
            let x8 = self.arg_to_u32_expr(meta, 0, 2);
            let x16 = self.arg_to_u32_expr(meta, 0, 4);

            let x2 = self.arg_to_u32_expr(meta, 1, 0);
            let x13 = self.arg_to_u32_expr(meta, 2, 0);
            let x22 = self.arg_to_u32_expr(meta, 3, 0);

            let x2_helper = nextn!(meta, self.aux.0, 1);
            let x2_helper_diff = nextn!(meta, self.aux.0, 2);
            let x13_helper = nextn!(meta, self.aux.0, 3);
            let x13_helper_diff = nextn!(meta, self.aux.0, 4);
            let x22_helper = nextn!(meta, self.aux.0, 5);
            let x22_helper_diff = nextn!(meta, self.aux.0, 6);

            let res = self.arg_to_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (x.clone() - x2 * constant_from!(1 << 2) - x2_helper.clone()),
                enable.clone() * (x2_helper + x2_helper_diff - constant_from!((1 << 2) - 1)),
                enable.clone() * (x8 - x13 * constant_from!(1 << 5) - x13_helper.clone()),
                enable.clone() * (x13_helper + x13_helper_diff - constant_from!((1 << 5) - 1)),
                enable.clone() * (x16 - x22 * constant_from!(1 << 6) - x22_helper.clone()),
                enable.clone() * (x22_helper + x22_helper_diff - constant_from!((1 << 6) - 1)),
                enable.clone() * (curr!(meta, self.op.0) - constant_from!(OP as i32)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcocde_expr(
                            curr!(meta, self.op.0),
                            vec![&x, &res],
                        )),
            ]
        });
    }
}
