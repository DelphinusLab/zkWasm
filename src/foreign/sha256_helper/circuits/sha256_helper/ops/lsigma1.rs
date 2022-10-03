use super::super::{Sha256HelperOp, Sha2HelperConfig};
use crate::{
    constant_from, curr, foreign::sha256_helper::circuits::sha256_helper::Sha2HelperEncode, nextn,
};
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};

const OP: Sha256HelperOp = Sha256HelperOp::LSigma1;

impl<F: FieldExt> Sha2HelperConfig<F> {
    pub(crate) fn configure_lsigma1(&self, meta: &mut ConstraintSystem<F>) {
        // (x >> 6) ^ (x >> 11) ^ (x >> 25)
        meta.create_gate("sha256 lsigma1", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_u32_expr(meta, 0, 0);
            let x8 = self.arg_to_u32_expr(meta, 0, 2);
            let x24 = self.arg_to_u32_expr(meta, 0, 6);

            let x6 = self.arg_to_u32_expr(meta, 1, 0);
            let x11 = self.arg_to_u32_expr(meta, 2, 0);
            let x25 = self.arg_to_u32_expr(meta, 3, 0);

            let x6_helper = nextn!(meta, self.aux.0, 1);
            let x6_helper_diff = nextn!(meta, self.aux.0, 2);
            let x11_helper = nextn!(meta, self.aux.0, 3);
            let x11_helper_diff = nextn!(meta, self.aux.0, 4);
            let x25_helper = nextn!(meta, self.aux.0, 5);
            let x25_helper_diff = nextn!(meta, self.aux.0, 6);

            let res = self.arg_to_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (x.clone() - x6 * constant_from!(1 << 6) - x6_helper.clone()),
                enable.clone()
                    * (x6_helper.clone() + x6_helper_diff - constant_from!((1 << 6) - 1)),
                enable.clone() * (x8 - x11 * constant_from!(1 << 3) - x11_helper.clone()),
                enable.clone() * (x11_helper + x11_helper_diff - constant_from!((1 << 3) - 1)),
                enable.clone() * (x24.clone() - x25 * constant_from!(1 << 1) - x25_helper.clone()),
                enable.clone() * (x25_helper + x25_helper_diff - constant_from!((1 << 1) - 1)),
                enable.clone() * (curr!(meta, self.op.0) - constant_from!(OP)),
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
