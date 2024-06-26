pub mod pair;
pub mod sum;
use ark_std::Zero;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::pairing::bn256::Fq as BN254Fq;
use halo2_proofs::pairing::bn256::Fq2 as BN254Fq2;
use halo2_proofs::pairing::bn256::Fr;
use halo2_proofs::pairing::bn256::G1Affine;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use std::ops::AddAssign;
use std::ops::Shl;

const LIMBSZ: usize = 54;
const LIMBNB: usize = 5;

use super::bn_to_field;
use super::field_to_bn;

fn fetch_fr(limbs: &[u64]) -> Fr {
    let mut bn = BigUint::zero();
    for (i, limb) in limbs.iter().enumerate().take(4) {
        bn.add_assign(BigUint::from_u64(*limb).unwrap() << (i * 64))
    }
    bn_to_field(&bn)
}

pub fn fetch_fq(limbs: &[u64], index: usize) -> BN254Fq {
    let mut bn = BigUint::zero();
    for i in 0..LIMBNB {
        bn.add_assign(BigUint::from_u64(limbs[index * LIMBNB + i]).unwrap() << (i * LIMBSZ))
    }
    bn_to_field(&bn)
}

pub fn fetch_fq2(limbs: &[u64], index: usize) -> BN254Fq2 {
    BN254Fq2 {
        c0: fetch_fq(limbs, index),
        c1: fetch_fq(limbs, index + 1),
    }
}

/// decode g1 from limbs where limbs[11] indicates whether the point is identity
fn fetch_g1(limbs: &Vec<u64>) -> G1Affine {
    assert_eq!(limbs.len(), LIMBNB * 2 + 1);
    let g1_identity = limbs[LIMBNB * 2];
    if g1_identity == 1 {
        G1Affine::generator()
    } else {
        let opt: Option<_> = G1Affine::from_xy(fetch_fq(limbs, 0), fetch_fq(limbs, 1)).into();
        opt.expect("from xy failed, not on curve")
    }
}

pub fn bn254_fq_to_limbs(result_limbs: &mut Vec<u64>, f: BN254Fq) {
    let mut bn = field_to_bn(&f);
    for _ in 0..LIMBNB {
        let d: BigUint = BigUint::from(1_u64).shl(LIMBSZ);
        let r = bn.clone() % d.clone();
        let value = if r == BigUint::from(0_u32) {
            0_u64
        } else {
            r.to_u64_digits()[0]
        };
        bn /= d;
        result_limbs.append(&mut vec![value]);
    }
}
