#![deny(unused_imports)]
#![deny(dead_code)]

pub mod bench;
pub mod circuits;
pub mod cli;
pub mod foreign;
pub mod runtime;
pub mod test;
pub mod traits;

#[macro_use]
extern crate lazy_static;
extern crate downcast_rs;
extern crate static_assertions;

// fn main() {
//     println!("Hello, world!");
// }
