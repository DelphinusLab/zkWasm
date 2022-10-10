use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr,
    foreign::sha256_helper::circuits::{assign::Sha256HelperTableChip, Sha2HelperEncode},
    nextn, rotation_constraints,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Region,
    plonk::{ConstraintSystem, Error},
};

const OP: Sha256HelperOp = Sha256HelperOp::SSigma0;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_ssigma0(&self, meta: &mut ConstraintSystem<F>) {
        // (x >> 7) ^ (x >> 18) ^ (x >> 3)

        meta.create_gate("sha256 ssigma0 opcode", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_rotate_u32_expr(meta, 0, 0);
            let res = self.arg_to_rotate_u32_expr(meta, 4, 0);

            vec![
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcode_expr(
                            curr!(meta, self.op.0),
                            vec![&res, &x],
                        )),
            ]
        });

        rotation_constraints!(meta, self, "ssigma0 rotate 7", 1, 7);
        rotation_constraints!(meta, self, "ssigma0 rotate 18", 2, 18);
        rotation_constraints!(meta, self, "ssigma0 rotate 3", 3, 3);
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_ssigma0(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 1, 7, 1)?;
        self.assign_rotate_aux(region, offset, args, 2, 18, 4)?;
        self.assign_rotate_aux(region, offset, args, 3, 3, 7)?;

        Ok(())
    }
}
