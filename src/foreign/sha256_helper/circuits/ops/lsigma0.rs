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

const OP: Sha256HelperOp = Sha256HelperOp::LSigma0;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_lsigma0(&self, meta: &mut ConstraintSystem<F>) {
        // (x right_rotate 2) ^ (x right_rotate 13) ^ (x right_rotate 22)
        sha256_constraints!(meta, self, "sha256 lsigma0 opcode");
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
