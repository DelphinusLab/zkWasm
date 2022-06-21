use halo2_proofs::{
    arithmetic::{BaseExt, FieldExt},
    circuit::Region,
};
use num_bigint::BigUint;

pub struct Context<'a, F: FieldExt> {
    pub region: Box<Region<'a, F>>,
    pub offset: usize,
}

pub fn field_to_bn<F: BaseExt>(f: &F) -> BigUint {
    let mut bytes: Vec<u8> = Vec::new();
    f.write(&mut bytes).unwrap();
    BigUint::from_bytes_le(&bytes[..])
}

pub fn bn_to_field<F: BaseExt>(bn: &BigUint) -> F {
    let mut bytes = bn.to_bytes_le();
    bytes.resize(32, 0);
    let mut bytes = &bytes[..];
    F::read(&mut bytes).unwrap()
}
