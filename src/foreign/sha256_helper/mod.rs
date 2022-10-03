use strum_macros::EnumIter;

pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;
pub mod test;

pub const SHA256_FOREIGN_TABLE_KEY: &'static str = "sha256-helper-table";

#[derive(Clone, Copy, EnumIter, PartialEq)]
pub enum Sha256HelperOp {
    Ch = 1,
    Maj = 2,
    LSigma0 = 3,
    LSigma1 = 4,
    SSigma0 = 5,
    SSigma1 = 6,
}
