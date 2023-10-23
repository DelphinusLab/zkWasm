pub mod sum;
use ark_std::Zero;
use halo2_proofs::pairing::bn256::Fr as BabyJubjubFq;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use std::ops::AddAssign;
use std::ops::Shl;
use zkwasm_host_circuits::host::jubjub;

const LIMBSZ: usize = 64;
const LIMBNB: usize = 4;

use super::bn_to_field;
use super::field_to_bn;

pub fn fetch_fq(limbs: &Vec<u64>, index: usize) -> BabyJubjubFq {
    let mut bn = BigUint::zero();
    for i in 0..LIMBNB {
        bn.add_assign(BigUint::from_u64(limbs[index * LIMBNB + i]).unwrap() << (i * LIMBSZ))
    }
    bn_to_field(&bn)
}

fn fetch_g1(limbs: &Vec<u64>) -> jubjub::Point {
    jubjub::Point {
        x: fetch_fq(limbs, 0),
        y: fetch_fq(limbs, 1),
    }
}

pub fn babyjubjub_fq_to_limbs(result_limbs: &mut Vec<u64>, f: BabyJubjubFq) {
    let mut bn = field_to_bn(&f);
    for _ in 0..LIMBNB {
        let d: BigUint = BigUint::from(1 as u64).shl(LIMBSZ);
        let r = bn.clone() % d.clone();
        let value = if r == BigUint::from(0 as u32) {
            0 as u64
        } else {
            r.to_u64_digits()[0]
        };
        bn = bn / d;
        result_limbs.append(&mut vec![value]);
    }
}
