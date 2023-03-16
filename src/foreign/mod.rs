use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{ConstraintSystem, Expression, VirtualCells},
};

pub mod keccak_helper;
pub mod log_helper;
pub mod require_helper;
pub mod sha256_helper;
#[cfg(not(feature = "v2"))]
pub mod v1;
#[cfg(feature = "v2")]
pub mod v2;
pub mod wasm_input_helper;

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    );
}
