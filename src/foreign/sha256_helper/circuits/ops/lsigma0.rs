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

const OP: Sha256HelperOp = Sha256HelperOp::LSigma0;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_lsigma0(&self, meta: &mut ConstraintSystem<F>) {
        // (x right_rotate 2) ^ (x right_rotate 13) ^ (x right_rotate 22)

        meta.create_gate("sha256 lsigma0 opcode", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_rotate_u32_expr(meta, 0, 0);
            let res = self.arg_to_rotate_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (curr!(meta, self.op.0) - constant_from!(OP)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcode_expr(
                            curr!(meta, self.op.0),
                            vec![x],
                            res
                        )),
            ]
        });

        rotation_constraints!(meta, self, "lsigma0 rotate 2", 1, 2);
        rotation_constraints!(meta, self, "lsigma0 rotate 13", 2, 13);
        rotation_constraints!(meta, self, "lsigma0 rotate 22", 3, 22);
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_lsigma0(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 1, 2, 1, false)?;
        self.assign_rotate_aux(region, offset, args, 2, 13, 4, false)?;
        self.assign_rotate_aux(region, offset, args, 3, 22, 7, false)?;

        Ok(())
    }
}
