// FIXME: tmp
#![allow(unused_imports, dead_code, unreachable_code, unused_must_use, unused_mut, unused_variables)]
#![deny(warnings)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![feature(int_roundings)]
#![feature(stmt_expr_attributes)]
#![feature(trait_upcasting)]

pub mod checksum;
pub mod circuits;
pub mod error;
pub mod foreign;
pub mod loader;
pub mod runtime;

mod profile;

#[cfg(test)]
pub mod test;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;

pub extern crate halo2_proofs;
pub extern crate zkwasm_host_circuits;
