use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr,
    foreign::sha256_helper::circuits::{assign::Sha256HelperTableChip, Sha2HelperEncode},
    nextn,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Region,
    plonk::{ConstraintSystem, Error},
};

const OP: Sha256HelperOp = Sha256HelperOp::SSigma1;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_ssigma1(&self, meta: &mut ConstraintSystem<F>) {
        // (x >> 17) ^ (x >> 19) ^ (x >> 10)
        meta.create_gate("sha256 ssigma1", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_u32_expr(meta, 0, 0);
            let x8 = self.arg_to_u32_expr(meta, 0, 2);
            let x16 = self.arg_to_u32_expr(meta, 0, 4);

            let x17 = self.arg_to_u32_expr(meta, 1, 0);
            let x19 = self.arg_to_u32_expr(meta, 2, 0);
            let x10 = self.arg_to_u32_expr(meta, 3, 0);

            let x17_helper = nextn!(meta, self.aux.0, 1);
            let x17_helper_diff = nextn!(meta, self.aux.0, 2);
            let x19_helper = nextn!(meta, self.aux.0, 3);
            let x19_helper_diff = nextn!(meta, self.aux.0, 4);
            let x10_helper = nextn!(meta, self.aux.0, 5);
            let x10_helper_diff = nextn!(meta, self.aux.0, 6);

            let res = self.arg_to_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (x16.clone() - x17 * constant_from!(1 << 1) - x17_helper.clone()),
                enable.clone()
                    * (x17_helper.clone() + x17_helper_diff - constant_from!((1 << 1) - 1)),
                enable.clone() * (x16 - x19 * constant_from!(1 << 3) - x19_helper.clone()),
                enable.clone() * (x19_helper + x19_helper_diff - constant_from!((1 << 3) - 1)),
                enable.clone() * (x8.clone() - x10 * constant_from!(1 << 2) - x10_helper.clone()),
                enable.clone() * (x10_helper + x10_helper_diff - constant_from!((1 << 2) - 1)),
                enable.clone() * (curr!(meta, self.op.0) - constant_from!(OP)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcode_expr(
                            curr!(meta, self.op.0),
                            vec![&res, &x],
                        )),
            ]
        });
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_ssigma1(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 1, 17, 1)?;
        self.assign_rotate_aux(region, offset, args, 2, 19, 3)?;
        self.assign_rotate_aux(region, offset, args, 3, 10, 5)?;

        Ok(())
    }
}
