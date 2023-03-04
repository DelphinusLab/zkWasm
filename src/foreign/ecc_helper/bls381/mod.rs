pub mod sum;
pub mod pair;
use num_bigint::BigUint;
use ark_std::Zero;
use std::ops::AddAssign;
use num_traits::FromPrimitive;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::pairing::bls12_381::{G1Affine,
    Fp2 as Bls381Fq2,
    Fq as Bls381Fq,
};

use super::{
    bn_to_field, field_to_bn
};

fn fetch_fq(limbs: &Vec<u64>, index:usize) -> Bls381Fq {
    let mut bn = BigUint::zero();
    for i in 0..8 {
        bn.add_assign(BigUint::from_u64(limbs[index * 8 + i]).unwrap() << (i * 54))
    }
    bn_to_field(&bn)
}

fn fetch_fq2(limbs: &Vec<u64>, index:usize) -> Bls381Fq2 {
    Bls381Fq2 {
        c0: fetch_fq(limbs,index),
        c1: fetch_fq(limbs, index+1),
    }
}

fn fetch_g1(limbs: &Vec<u64>, g1_identity: bool) -> G1Affine {
    if g1_identity {
        G1Affine::identity()
    } else {
        let opt:Option<_> = G1Affine::from_xy(
            fetch_fq(limbs,0),
            fetch_fq(limbs,1)
        ).into();
        opt.expect("from xy failed, not on curve")
    }
}

fn bls381_fq_to_limbs(result_limbs: &mut Vec<u64>, f: Bls381Fq) {
    let mut bn = field_to_bn(&f);
    for _ in 0..8 {
        let d:BigUint = BigUint::from(1u64 << 54);
        let r = bn.clone() % d.clone();
        let value = if r == BigUint::from(0 as u32) {
            0 as u64
        } else {
            r.to_u64_digits()[0]
        };
        bn = bn / d;
        result_limbs.append(&mut vec![value]);
    };
}

