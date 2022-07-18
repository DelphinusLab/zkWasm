use num_bigint::BigUint;

pub mod config_builder;
pub mod etable;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod rtable;
pub mod utils;

trait Encode {
    fn encode(&self) -> BigUint;
}
