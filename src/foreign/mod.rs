use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::etable::allocator::EventTableCellAllocator;
use crate::circuits::etable::constraint_builder::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

enum ForeignInst {
    Log = 0,
    BlspairG1,
    BlspairG2,
    BlspairG3,
    BlsSumG1,
    BlsSumResult,
    Bn254SumG1,
    Bn254SumResult,
    Bn254PairG1,
    Bn254PairG2,
    Bn254PairG3,
    KVPairAddr,
    KVPairSet,
    KVPairGet,
}

pub mod keccak_helper;
pub mod log_helper;
pub mod require_helper;
pub mod kv_helper;
pub mod ecc_helper;
pub mod wasm_input_helper;

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Vec<Expression<F>>,
    );
}

pub(crate) trait EventTableForeignCallConfigBuilder<F: FieldExt> {
    fn configure(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        lookup_cells: &mut (impl Iterator<Item = AllocatedUnlimitedCell<F>> + Clone),
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub(crate) trait InternalHostPluginBuilder {
    fn new(index: usize) -> Self;
}
