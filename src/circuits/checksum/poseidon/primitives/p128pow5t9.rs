use std::marker::PhantomData;

use halo2_proofs::arithmetic::FieldExt;

use super::generate_constants;
use super::Mds;
use super::Spec;

pub(crate) const WIDTH: usize = 9;
pub(crate) const RATE: usize = 8;

/// Poseidon-128 using the $x^5$ S-box, with a width of 3 field elements, and the
/// standard number of rounds for 128-bit security "with margin".
///
/// The standard specification for this set of parameters (on either of the Pasta
/// fields) uses $R_F = 8, R_P = 56$. This is conveniently an even number of
/// partial rounds, making it easier to construct a Halo 2 circuit.
#[derive(Debug)]
pub struct P128Pow5T9<F: FieldExt> {
    _mark: PhantomData<F>,
}

impl<F: FieldExt> Spec<F, 9, 8> for P128Pow5T9<F> {
    fn full_rounds() -> usize {
        8
    }

    fn partial_rounds() -> usize {
        64
    }

    fn sbox(val: F) -> F {
        val.pow_vartime([5])
    }

    fn secure_mds() -> usize {
        0
    }

    fn constants() -> (Vec<[F; 9]>, Mds<F, 9>, Mds<F, 9>) {
        generate_constants::<F, Self, 9, 8>()
    }
}
