use super::{assign::Sha256HelperTableChip, BLOCK_LINES};
use halo2_proofs::{arithmetic::FieldExt, circuit::Region, plonk::Error};

pub mod ch;
pub mod lsigma0;
pub mod lsigma1;
pub mod maj;
pub mod ssigma0;
pub mod ssigma1;

impl<F: FieldExt> Sha256HelperTableChip<F> {
    pub(crate) fn assign_rotate_aux(
        &self,
        region: &mut Region<F>,
        offset: usize,
        args: &Vec<u32>,
        index: usize,
        rotate_bits: u32,
        start: usize,
    ) -> Result<(), Error> {
        let value = args[0].rotate_right(rotate_bits);
        let u4_shift = rotate_bits / 4;
        let u4_inner_shift = rotate_bits % 4;
        let modulus_mask = (1 << (rotate_bits % 4)) - 1;

        let u4_shifted_args0 = args[0] >> (u4_shift * 4);
        let round = (u4_shifted_args0 & 0xf) >> u4_inner_shift;
        let rem = u4_shifted_args0 & modulus_mask;
        let diff = modulus_mask - rem;

        region.assign_advice(
            || "sha256 helper rotate round",
            self.config.aux.0,
            offset + start,
            || Ok(F::from(round as u64)),
        )?;

        region.assign_advice(
            || "sha256 helper rotate rem",
            self.config.aux.0,
            offset + start + 1,
            || Ok(F::from(rem as u64)),
        )?;

        region.assign_advice(
            || "sha256 helper rotate diff",
            self.config.aux.0,
            offset + start + 2,
            || Ok(F::from(diff as u64)),
        )?;

        for i in 0..8 {
            region.assign_advice(
                || "sha256 rotate value",
                self.config.args[index].0,
                offset + i,
                || Ok(F::from(((value as u64) >> (4 * i)) & 0xf)),
            )?;
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! rotation_constraints {
    ($meta:expr, $self:expr, $key:expr, $index:expr, $rotate:expr) => {
        $meta.create_gate($key, |meta| {
            let enable = $self.is_op_enabled_expr(meta, OP);

            let (x, x_lowest) = $self.arg_to_rotate_u32_expr_with_lowest_u4(meta, 0, $rotate / 4);
            let y = $self.arg_to_rotate_u32_expr(meta, $index, 0);

            let round = nextn!(meta, $self.aux.0, $index * 3 - 2);
            let rem = nextn!(meta, $self.aux.0, $index * 3 - 1);
            let rem_diff = nextn!(meta, $self.aux.0, $index * 3);

            vec![
                enable.clone()
                    * (x_lowest - round * constant_from!(1 << ($rotate % 4)) - rem.clone()),
                enable.clone()
                    * (rem.clone() + rem_diff.clone() - constant_from!((1 << ($rotate % 4)) - 1)),
                enable.clone()
                    * (y * constant_from!(1 << ($rotate % 4))
                        - rem.clone() * constant_from!(1u64 << 32)
                        + rem
                        - x),
            ]
        });
    };
}
