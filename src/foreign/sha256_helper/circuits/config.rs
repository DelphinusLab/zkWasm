use super::{Sha2HelperConfig, OP_ARGS_NUM};
use crate::foreign::sha256_helper::Sha256HelperOp;
use crate::{constant_from, curr, fixed_curr, next, nextn};
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};
use strum::IntoEnumIterator;

impl<F: FieldExt> Sha2HelperConfig<F> {
    pub fn _configure(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("sha256 helper op_bits sum equals to 1", |meta| {
            let sum = Sha256HelperOp::iter()
                .map(|op| nextn!(meta, self.op_bit.0, op as i32))
                .reduce(|acc, expr| acc + expr)
                .unwrap();

            vec![fixed_curr!(meta, self.block_first_line_sel) * (constant_from!(1) - sum)]
        });

        meta.create_gate("sha256 op eq inside a block", |meta| {
            vec![self.is_not_block_end_expr(meta) * (next!(meta, self.op.0) - curr!(meta, self.op.0))]
        });

        meta.lookup("sha256 op lookup", |meta| {
            let mut pos_acc = 1 << (4 * OP_ARGS_NUM);
            let mut acc = curr!(meta, self.op.0) * constant_from!(pos_acc);

            for i in 0..OP_ARGS_NUM {
                pos_acc >>= 4;
                acc = acc + curr!(meta, self.args[i].0) * constant_from!(pos_acc);
            }

            vec![(fixed_curr!(meta, self.sel) * acc, self.op_valid_set)]
        });

        self.configure_ssigma0(meta);
        self.configure_ssigma1(meta);
        self.configure_lsigma0(meta);
        self.configure_lsigma1(meta);
        self.configure_ch(meta);
        self.configure_maj(meta);
    }
}
