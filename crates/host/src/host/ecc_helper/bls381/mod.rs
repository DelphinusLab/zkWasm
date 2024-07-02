pub mod pair;
pub mod sum;
use ark_std::Zero;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::pairing::bls12_381::Fp2 as Bls381Fq2;
use halo2_proofs::pairing::bls12_381::Fq as Bls381Fq;
use halo2_proofs::pairing::bls12_381::G1Affine;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use std::ops::AddAssign;

use super::bn_to_field;
use super::field_to_bn;

fn fetch_fq(limbs: &[u64], index: usize) -> Bls381Fq {
    let mut bn = BigUint::zero();
    for i in 0..8 {
        bn.add_assign(BigUint::from_u64(limbs[index * 8 + i]).unwrap() << (i * 54))
    }
    bn_to_field(&bn)
}

fn fetch_fq2(limbs: &[u64], index: usize) -> Bls381Fq2 {
    Bls381Fq2 {
        c0: fetch_fq(limbs, index),
        c1: fetch_fq(limbs, index + 1),
    }
}

fn fetch_g1(limbs: &[u64], g1_identity: bool) -> G1Affine {
    if g1_identity {
        G1Affine::identity()
    } else {
        let opt: Option<_> = G1Affine::from_xy(fetch_fq(limbs, 0), fetch_fq(limbs, 1)).into();
        opt.expect("from xy failed, not on curve")
    }
}

fn bls381_fq_to_limbs(result_limbs: &mut Vec<u64>, f: Bls381Fq) {
    let mut bn = field_to_bn(&f);
    for _ in 0..8 {
        let d: BigUint = BigUint::from(1u64 << 54);
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
