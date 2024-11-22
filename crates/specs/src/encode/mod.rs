use std::ops::Add;
use std::ops::Mul;

use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Expression;
use num_bigint::BigUint;

pub mod br_table;
pub mod frame_table;
pub mod image_table;
pub mod init_memory_table;
pub mod instruction_table;
pub mod memory_table;
pub mod opcode;

pub(crate) const U32_BITS: u32 = 32;

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

fn bn_to_field<F: FieldExt>(bn: &BigUint) -> F {
    let mut bytes = bn.to_bytes_le();
    bytes.resize(32, 0);
    let mut bytes = &bytes[..];
    F::read(&mut bytes).unwrap()
}

impl<F: FieldExt> FromBn for Expression<F> {
    fn from_bn(bn: &BigUint) -> Self {
        halo2_proofs::plonk::Expression::Constant(bn_to_field(bn))
    }

    fn zero() -> Self {
        halo2_proofs::plonk::Expression::Constant(F::zero())
    }
}
