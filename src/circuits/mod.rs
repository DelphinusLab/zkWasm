use num_bigint::BigUint;

pub mod etable;
pub mod imtable;
pub mod itable;
pub mod jtable;
pub mod mtable;
pub mod rtable;
pub mod utils;
pub mod config_builder;


trait Encode {
    fn encode(&self) -> BigUint;
}
