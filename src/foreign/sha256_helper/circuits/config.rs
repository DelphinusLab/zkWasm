use super::{Sha256HelperTableConfig, Sha2HelperEncode};
use crate::foreign::sha256_helper::Sha256HelperOp;
use crate::{constant_from, curr, fixed_curr, next, nextn};
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};
use strum::IntoEnumIterator;

impl<F: FieldExt> Sha256HelperTableConfig<F> {
    pub fn _configure(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("sha256 helper op_bits sum equals to 1", |meta| {
            let sum = Sha256HelperOp::iter()
                .map(|op| nextn!(meta, self.op_bit.0, op as i32))
                .reduce(|acc, expr| acc + expr)
                .unwrap();

            vec![
                fixed_curr!(meta, self.block_first_line_sel)
                    * (constant_from!(1) - sum)
                    * self.is_block_enabled_expr(meta),
            ]
        });

        meta.create_gate("sha256 op eq inside a block", |meta| {
            vec![
                self.is_not_block_end_expr(meta)
                    * (next!(meta, self.op.internal) - curr!(meta, self.op.internal)),
            ]
        });

        meta.lookup("sha256 op lookup", |meta| {
            vec![(
                fixed_curr!(meta, self.sel)
                    * Sha2HelperEncode::encode_table_expr(
                        curr!(meta, self.op.internal),
                        [
                            curr!(meta, self.args[1].internal),
                            curr!(meta, self.args[2].internal),
                            curr!(meta, self.args[3].internal),
                        ],
                        curr!(meta, self.args[4].internal),
                    ),
                self.op_valid_set,
            )]
        });

        self.configure_ssigma0(meta);
        self.configure_ssigma1(meta);
        self.configure_lsigma0(meta);
        self.configure_lsigma1(meta);
        self.configure_ch(meta);
        self.configure_maj(meta);
    }
}
