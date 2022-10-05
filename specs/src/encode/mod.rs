use std::ops::{Add, Mul, Shl};

use halo2_proofs::{arithmetic::FieldExt, plonk::Expression, transcript::bn_to_field};
use num_bigint::BigUint;

pub mod opcode;

pub trait FromBn: Sized + Add<Self, Output = Self> + Mul<Self, Output = Self> {
    fn zero() -> Self;
    fn from_bn(bn: &BigUint) -> Self;
}

impl FromBn for BigUint {
    fn zero() -> Self {
        BigUint::from(0u64)
    }

    fn from_bn(bn: &BigUint) -> Self {
        bn.clone()
    }
}

impl<F: FieldExt> FromBn for Expression<F> {
    fn from_bn(bn: &BigUint) -> Self {
        halo2_proofs::plonk::Expression::Constant(bn_to_field(bn))
    }

    fn zero() -> Self {
        halo2_proofs::plonk::Expression::Constant(F::zero())
    }
}
