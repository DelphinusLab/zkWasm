use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::etable::allocator::EventTableCellAllocator;
use crate::circuits::etable::constraint_builder::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::runtime::host::host_env::HostEnv;
use crate::runtime::wasmi_interpreter::WasmRuntimeIO;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

use self::log_helper::register_log_foreign;
use self::require_helper::register_require_foreign;
use self::wasm_input_helper::runtime::register_wasm_input_foreign;

pub mod keccak_helper;
pub mod log_helper;
pub mod require_helper;
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

impl HostEnv {
    pub fn new_with_full_foreign_plugins(
        public_inputs: Vec<u64>,
        private_inputs: Vec<u64>,
    ) -> (Self, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, public_inputs, private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        env.finalize();

        (env, wasm_runtime_io)
    }
}
