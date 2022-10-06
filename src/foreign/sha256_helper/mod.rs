use strum_macros::EnumIter;

pub mod circuits;
pub mod etable_op_configure;
pub mod runtime;
pub mod test;

pub const SHA256_FOREIGN_TABLE_KEY: &'static str = "sha256-helper-table";
pub const SHA256_FOREIGN_FUNCTION_NAME_MAJ: &'static str = "zkwasm_sha256_maj";
pub const SHA256_FOREIGN_FUNCTION_NAME_CH: &'static str = "zkwasm_sha256_ch";
pub const SHA256_FOREIGN_FUNCTION_NAME_SSIGMA0: &'static str = "zkwasm_sha256_ssigma0";
pub const SHA256_FOREIGN_FUNCTION_NAME_SSIGMA1: &'static str = "zkwasm_sha256_ssigma1";
pub const SHA256_FOREIGN_FUNCTION_NAME_LSIGMA0: &'static str = "zkwasm_sha256_lsigma0";
pub const SHA256_FOREIGN_FUNCTION_NAME_LSIGMA1: &'static str = "zkwasm_sha256_lsigma1";

#[derive(Clone, Copy, EnumIter, PartialEq)]
pub enum Sha256HelperOp {
    Ch = 1,
    Maj = 2,
    LSigma0 = 3,
    LSigma1 = 4,
    SSigma0 = 5,
    SSigma1 = 6,
}
