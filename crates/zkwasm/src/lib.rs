#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_imports)]
#![feature(thread_local)]
#![feature(variant_count)]

pub mod circuits;
pub mod foreign;
pub mod loader;
pub mod runtime;

#[cfg(feature = "checksum")]
pub mod image_hasher;

mod profile;

#[cfg(test)]
pub mod test;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;

pub extern crate halo2_proofs;
pub extern crate halo2aggregator_s;
pub extern crate zkwasm_host_circuits;
