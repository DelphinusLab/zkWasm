use halo2_proofs::arithmetic::Field;
use halo2_proofs::pairing::bn256::Fr;

use super::generate_constants;
use super::Mds;
use super::Spec;

/// Poseidon-128 using the $x^5$ S-box, with a width of 3 field elements, and the
/// standard number of rounds for 128-bit security "with margin".
///
/// The standard specification for this set of parameters (on either of the Pasta
/// fields) uses $R_F = 8, R_P = 56$. This is conveniently an even number of
/// partial rounds, making it easier to construct a Halo 2 circuit.
#[derive(Debug)]
pub struct P128Pow5T9;

impl Spec<Fr, 9, 8> for P128Pow5T9 {
    fn full_rounds() -> usize {
        8
    }

    fn partial_rounds() -> usize {
        64
    }

    fn sbox(val: Fr) -> Fr {
        val.pow_vartime([5])
    }

    fn secure_mds() -> usize {
        0
    }

    fn constants() -> (Vec<[Fr; 9]>, Mds<Fr, 9>, Mds<Fr, 9>) {
        generate_constants::<Fr, Self, 9, 8>()
    }
}
