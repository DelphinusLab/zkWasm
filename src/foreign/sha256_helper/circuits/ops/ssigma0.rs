use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr, foreign::sha256_helper::circuits::{Sha2HelperEncode, assign::Sha256HelperTableChip}, nextn,
};
use halo2_proofs::{arithmetic::FieldExt, plonk::{ConstraintSystem, Error}, circuit::Region};

const OP: Sha256HelperOp = Sha256HelperOp::SSigma0;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_ssigma0(&self, meta: &mut ConstraintSystem<F>) {
        // (x >> 7) ^ (x >> 18) ^ (x >> 3)
        meta.create_gate("sha256 ssigma0", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_u32_expr(meta, 0, 0);
            let x16 = self.arg_to_u32_expr(meta, 0, 4);

            let x7 = self.arg_to_u32_expr(meta, 1, 0);
            let x18 = self.arg_to_u32_expr(meta, 2, 0);
            let x3 = self.arg_to_u32_expr(meta, 3, 0);

            let x7_helper = nextn!(meta, self.aux.0, 1);
            let x7_helper_diff = nextn!(meta, self.aux.0, 2);
            let x18_helper = nextn!(meta, self.aux.0, 3);
            let x18_helper_diff = nextn!(meta, self.aux.0, 4);
            let x3_helper = nextn!(meta, self.aux.0, 5);
            let x3_helper_diff = nextn!(meta, self.aux.0, 6);

            let res = self.arg_to_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (x.clone() - x7 * constant_from!(1 << 7) - x7_helper.clone()),
                enable.clone()
                    * (x7_helper.clone() + x7_helper_diff - constant_from!((1 << 7) - 1)),
                enable.clone() * (x16 - x18 * constant_from!(1 << 2) - x18_helper.clone()),
                enable.clone() * (x18_helper + x18_helper_diff - constant_from!((1 << 2) - 1)),
                enable.clone() * (x.clone() - x3 * constant_from!(1 << 3) - x3_helper.clone()),
                enable.clone() * (x3_helper + x3_helper_diff - constant_from!((1 << 3) - 1)),
                enable.clone() * (curr!(meta, self.op.0) - constant_from!(OP)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcocde_expr(
                            curr!(meta, self.op.0),
                            vec![&res, &x],
                        )),
            ]
        });
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_ssigma0(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 7, 1)?;
        self.assign_rotate_aux(region, offset, args, 18, 3)?;
        self.assign_rotate_aux(region, offset, args, 3, 5)?;

        Ok(())
    }
}
