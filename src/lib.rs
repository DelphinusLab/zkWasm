#![deny(unused_imports)]
#![deny(dead_code)]

pub mod bench;
pub mod circuits;
pub mod cli;
pub mod foreign;
pub mod runtime;
pub mod traits;

#[cfg(test)]
pub mod test;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;

// fn main() {
//     println!("Hello, world!");
// }
