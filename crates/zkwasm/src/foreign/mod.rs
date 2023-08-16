use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::config::zkwasm_k;
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
use crate::foreign::log_helper::register_external_output_foreign;

use self::context::runtime::register_context_foreign;
use self::log_helper::register_log_foreign;
use self::require_helper::register_require_foreign;
use self::wasm_input_helper::runtime::register_wasm_input_foreign;
use self::ecc_helper::{
    bls381::pair::register_blspair_foreign,
    bls381::sum::register_blssum_foreign,
    bn254::pair::register_bn254pair_foreign,
    bn254::sum::register_bn254sum_foreign,
    jubjub::sum::register_babyjubjubsum_foreign,
};
use self::hash_helper::sha256::register_sha256_foreign;
use self::hash_helper::poseidon::register_poseidon_foreign;
use self::merkle_helper::merkle::register_merkle_foreign;

pub mod context;
pub mod keccak_helper;
pub mod log_helper;
pub mod require_helper;
pub mod merkle_helper;
pub mod ecc_helper;
pub mod hash_helper;
pub mod wasm_input_helper;

pub fn foreign_table_enable_lines() -> usize {
    1 << (zkwasm_k() - 1)
}

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        name: &'static str,
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
        context_input: Vec<u64>,
        context_output: Rc<RefCell<Vec<u64>>>,
        external_output: Rc<RefCell<HashMap<u64, Vec<u64>>>>
    ) -> (Self, WasmRuntimeIO) {
        let mut env = HostEnv::new();
        let wasm_runtime_io = register_wasm_input_foreign(&mut env, public_inputs, private_inputs);
        register_require_foreign(&mut env);
        register_log_foreign(&mut env);
        register_merkle_foreign(&mut env);
        register_blspair_foreign(&mut env);
        register_blssum_foreign(&mut env);
        register_bn254pair_foreign(&mut env);
        register_bn254sum_foreign(&mut env);
        register_sha256_foreign(&mut env);
        register_poseidon_foreign(&mut env);
        register_babyjubjubsum_foreign(&mut env);
        register_context_foreign(&mut env, context_input, context_output);
        register_external_output_foreign(&mut env, external_output);
        env.finalize();

        (env, wasm_runtime_io)
    }
}
