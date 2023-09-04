#![deny(dead_code)]
#![deny(unused_variables)]
#![deny(unused_imports)]
#![feature(thread_local)]

pub mod checksum;
pub mod circuits;
pub mod foreign;
pub mod loader;
pub mod runtime;

mod profile;

#[cfg(test)]
pub mod test;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;
