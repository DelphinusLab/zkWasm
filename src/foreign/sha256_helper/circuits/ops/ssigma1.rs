use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr,
    foreign::sha256_helper::circuits::{assign::Sha256HelperTableChip, Sha2HelperEncode},
    nextn, rotation_constraints, shift_constraints,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Region,
    plonk::{ConstraintSystem, Error},
};

const OP: Sha256HelperOp = Sha256HelperOp::SSigma1;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_ssigma1(&self, meta: &mut ConstraintSystem<F>) {
        // (x right_rotate 17) ^ (x right_rotate 19) ^ (x >> 10)

        meta.create_gate("sha256 ssigma1 opcode", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let x = self.arg_to_rotate_u32_expr(meta, 0, 0);
            let res = self.arg_to_rotate_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (curr!(meta, self.op) - constant_from!(OP)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcode_expr(curr!(meta, self.op), vec![x], res)),
            ]
        });

        rotation_constraints!(meta, self, "ssigma1 rotate 17", 1, 17);
        rotation_constraints!(meta, self, "ssigma1 rotate 19", 2, 19);
        shift_constraints!(meta, self, "ssigma1 shift 10", 3, 10);
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_ssigma1(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 1, 17, 1, false)?;
        self.assign_rotate_aux(region, offset, args, 2, 19, 4, false)?;
        self.assign_rotate_aux(region, offset, args, 3, 10, 7, true)?;

        Ok(())
    }
}
