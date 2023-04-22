pub mod sum;
pub mod pair;
use num_bigint::BigUint;
use halo2_proofs::pairing::bn256::{
    Fq2 as BN254Fq2,
    Fq as BN254Fq,
    G1Affine,
};
use ark_std::Zero;
use std::ops::{AddAssign, Shl};
use num_traits::FromPrimitive;
use halo2_proofs::arithmetic::CurveAffine;

const BN254SUM_G1:usize = 5;
const BN254SUM_RESULT:usize = 6;
const BN254PAIR_G1:usize = 7;
const BN254PAIR_G2:usize = 8;
const BN254PAIR_G3:usize = 9;

const LIMBSZ:usize = 45;
const LIMBNB:usize = 6;

use super::{bn_to_field, field_to_bn};

pub fn fetch_fq(limbs: &Vec<u64>, index:usize) -> BN254Fq {
    let mut bn = BigUint::zero();
    for i in 0..LIMBNB {
        bn.add_assign(BigUint::from_u64(limbs[index * LIMBNB + i]).unwrap() << (i * LIMBSZ))
    }
    bn_to_field(&bn)
}

pub fn fetch_fq2(limbs: &Vec<u64>, index:usize) -> BN254Fq2 {
    BN254Fq2 {
        c0: fetch_fq(limbs,index),
        c1: fetch_fq(limbs, index+1),
    }
}

fn fetch_g1(limbs: &Vec<u64>, g1_identity: bool) -> G1Affine {
    if g1_identity {
        G1Affine::generator()
    } else {
        let opt:Option<_> = G1Affine::from_xy(
            fetch_fq(limbs,0),
            fetch_fq(limbs,1)
        ).into();
        opt.expect("from xy failed, not on curve")
    }
}

pub fn bn254_fq_to_limbs(result_limbs: &mut Vec<u64>, f: BN254Fq) {
    let mut bn = field_to_bn(&f);
    for _ in 0..LIMBNB {
        let d:BigUint = BigUint::from(1 as u64).shl(LIMBSZ);
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
