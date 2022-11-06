use std::{cell::RefCell, rc::Rc};

use halo2_proofs::{
    arithmetic::{BaseExt, FieldExt},
    circuit::Region,
};
use num_bigint::BigUint;

pub mod bitvalue;
pub mod bytes8;
pub mod largrange;
pub mod row_diff;
pub mod u16;
pub mod u32;
pub mod u64;
pub mod u8;

pub struct Context<'a, F: FieldExt> {
    pub region: Rc<RefCell<Region<'a, F>>>,
    pub offset: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    records: Vec<usize>,
}

impl<'a, F: FieldExt> Context<'a, F> {
    pub fn new(region: Rc<RefCell<Region<'a, F>>>, start_offset: usize, end_offset: usize) -> Self {
        Self {
            region,
            offset: start_offset,
            start_offset,
            end_offset,
            records: vec![],
        }
    }

    pub fn next(&mut self) {
        self.offset += 1;

        assert!(self.offset <= self.end_offset);
    }

    pub fn reset(&mut self) {
        self.offset = self.start_offset;
        self.records.clear();
    }

    pub fn push(&mut self) {
        self.records.push(self.offset)
    }

    pub fn pop(&mut self) {
        self.offset = self.records.pop().unwrap();
    }
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

#[macro_export]
macro_rules! curr {
    ($meta: expr, $x: expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation::cur())
    };
}

#[macro_export]
macro_rules! prev {
    ($meta: expr, $x: expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation::prev())
    };
}

#[macro_export]
macro_rules! next {
    ($meta: expr, $x: expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation::next())
    };
}

#[macro_export]
macro_rules! nextn {
    ($meta: expr, $x: expr, $n:expr) => {
        $meta.query_advice($x, halo2_proofs::poly::Rotation($n))
    };
}

#[macro_export]
macro_rules! fixed_curr {
    ($meta: expr, $x: expr) => {
        $meta.query_fixed($x, halo2_proofs::poly::Rotation::cur())
    };
}

#[macro_export]
macro_rules! instance_curr {
    ($meta: expr, $x: expr) => {
        $meta.query_instance($x, halo2_proofs::poly::Rotation::cur())
    };
}

#[macro_export]
macro_rules! fixed_prev {
    ($meta: expr, $x: expr) => {
        $meta.query_fixed($x, halo2_proofs::poly::Rotation::prev())
    };
}

#[macro_export]
macro_rules! fixed_next {
    ($meta: expr, $x: expr) => {
        $meta.query_fixed($x, halo2_proofs::poly::Rotation::next())
    };
}

#[macro_export]
macro_rules! constant_from {
    ($x: expr) => {
        halo2_proofs::plonk::Expression::Constant(F::from($x as u64))
    };
}

#[macro_export]
macro_rules! constant_from_bn {
    ($x: expr) => {
        halo2_proofs::plonk::Expression::Constant(bn_to_field($x))
    };
}

#[macro_export]
macro_rules! constant {
    ($x: expr) => {
        halo2_proofs::plonk::Expression::Constant($x)
    };
}
