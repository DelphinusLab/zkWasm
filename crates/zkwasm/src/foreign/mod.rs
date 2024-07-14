use crate::circuits::cell::AllocatedUnlimitedCell;
use crate::circuits::cell::CellExpression;
use crate::circuits::etable::allocator::EventTableCellAllocator;
use crate::circuits::etable::constraint_builder::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;

pub mod context;
pub mod log_helper;
pub mod require_helper;
pub mod wasm_input_helper;

pub fn foreign_table_enable_lines(k: u32) -> usize {
    1 << (k as usize - 1)
}

pub trait ForeignTableConfig<F: FieldExt> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        name: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Vec<Expression<F>>,
    );
}

pub(crate) trait EventTableForeignCallConfigBuilder<F: FieldExt>: Sized {
    fn configure_all(
        self,
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        lookup_cells: &mut (impl Iterator<Item = AllocatedUnlimitedCell<F>> + Clone),
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let unused_args = common_config
            .uniarg_configs
            .iter()
            .map(|x| x.is_enabled_cell)
            .collect::<Vec<_>>();
        constraint_builder.push(
            "foreign call: uniarg",
            Box::new(move |meta| {
                vec![unused_args
                    .iter()
                    .map(|x| x.expr(meta))
                    .reduce(|a, b| a + b)
                    .unwrap()]
            }),
        );

        let mut common_config = common_config.clone();
        common_config.uniarg_configs = common_config.uniarg_configs.into_iter().take(0).collect();
        self.configure(&common_config, allocator, constraint_builder, lookup_cells)
    }

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
