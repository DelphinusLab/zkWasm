use super::super::{Sha256HelperOp, Sha256HelperTableConfig};
use crate::{
    constant_from, curr,
    foreign::sha256_helper::circuits::{assign::Sha256HelperTableChip, Sha2HelperEncode},
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Region,
    plonk::{ConstraintSystem, Error},
};

const OP: Sha256HelperOp = Sha256HelperOp::Ch;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub(crate) fn configure_ch(&self, meta: &mut ConstraintSystem<F>) {
        // (e & f) ^ (!e & g)
        meta.create_gate("sha256 ch", |meta| {
            let enable = self.is_op_enabled_expr(meta, OP);

            let e = self.arg_to_rotate_u32_expr(meta, 1, 0);
            let f = self.arg_to_rotate_u32_expr(meta, 2, 0);
            let g = self.arg_to_rotate_u32_expr(meta, 3, 0);
            let res = self.arg_to_rotate_u32_expr(meta, 4, 0);

            vec![
                enable.clone() * (curr!(meta, self.op) - constant_from!(OP)),
                enable.clone()
                    * (self.opcode_expr(meta)
                        - Sha2HelperEncode::encode_opcode_expr(
                            curr!(meta, self.op),
                            vec![e, f, g],
                            res,
                        )),
            ]
        });
    }
}

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_ch(
        &self,
        _region: &mut Region<F>,
        _offset: usize,
        _args: &Vec<u32>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
