use strum_macros::EnumIter;

pub mod circuits;
pub mod etable_op_configure;

#[derive(Clone, Copy, EnumIter, PartialEq)]
pub enum Sha256HelperOp {
    Ch = 1,
    Maj = 2,
    LSigma0 = 3,
    LSigma1 = 4,
    SSigma0 = 5,
    SSigma1 = 6,
}
