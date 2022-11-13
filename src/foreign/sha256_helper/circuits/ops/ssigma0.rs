use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr,
    foreign::sha256_helper::circuits::{assign::Sha256HelperTableChip, Sha2HelperEncode},
    nextn, rotation_constraints, sha256_sigma_common_constraints, shift_constraints,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Region,
    plonk::{ConstraintSystem, Error},
};

const OP: Sha256HelperOp = Sha256HelperOp::SSigma0;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_ssigma0(&self, meta: &mut ConstraintSystem<F>) {
        // (x right_rotate 7) ^ (x right_rotate 18) ^ (x >> 3)

        sha256_sigma_common_constraints!(meta, self, "sha256 ssigma0 opcode");
        rotation_constraints!(meta, self, "ssigma0 rotate 7", 1, 7);
        rotation_constraints!(meta, self, "ssigma0 rotate 18", 2, 18);
        shift_constraints!(meta, self, "ssigma0 shift 3", 3, 3);
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_ssigma0(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 1, 7, 1, false)?;
        self.assign_rotate_aux(region, offset, args, 2, 18, 4, false)?;
        self.assign_rotate_aux(region, offset, args, 3, 3, 7, true)?;

        Ok(())
    }
}
