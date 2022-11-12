use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr,
    foreign::sha256_helper::circuits::{assign::Sha256HelperTableChip, Sha2HelperEncode},
    nextn, rotation_constraints, sha256_constraints,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Region,
    plonk::{ConstraintSystem, Error},
};

const OP: Sha256HelperOp = Sha256HelperOp::LSigma1;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_lsigma1(&self, meta: &mut ConstraintSystem<F>) {
        // (x right_rotate 6) ^ (x right_rotate 11) ^ (x right_rotate 25)
        sha256_constraints!(meta, self, "sha256 lsigma1 opcode");
        rotation_constraints!(meta, self, "lsigma1 rotate 6", 1, 6);
        rotation_constraints!(meta, self, "lsigma1 rotate 11", 2, 11);
        rotation_constraints!(meta, self, "lsigma1 rotate 25", 3, 25);
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_lsigma1(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
    ) -> Result<(), Error> {
        self.assign_rotate_aux(region, offset, args, 1, 6, 1, false)?;
        self.assign_rotate_aux(region, offset, args, 2, 11, 4, false)?;
        self.assign_rotate_aux(region, offset, args, 3, 25, 7, false)?;

        Ok(())
    }
}
