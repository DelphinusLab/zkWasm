use super::*;
use crate::{
    circuits::{
        intable::InputTableEncode,
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant,
};

use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use specs::{
    etable::EventTableEntry,
    host_function::HostFunction,
    itable::{HostPlugin, OpcodeClass, OPCODE_CLASS_SHIFT},
};
use specs::{mtable::VarType, step::StepInfo};

pub struct CallHostSha256Config {}

pub struct CallHostSha256ConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallHostSha256ConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        _constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        todo!();

        Box::new(CallHostSha256Config {})
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallHostSha256Config {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        todo!()
        /* TODO: add function index
        constant!(bn_to_field(
            &(BigUint::from(OpcodeClass::Sha256 as u64) << OPCODE_CLASS_SHIFT)
        ))
        */
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step_info: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error> {
        match &entry.step_info {
            StepInfo::CallHost {
                plugin,
                host_function_idx,
                args,
                ret_val,
                signature,
            } => {
                assert_eq!(*plugin, HostPlugin::Sha256);

                todo!()
            }

            _ => unreachable!(),
        }
    }

    fn opcode_class(&self) -> OpcodeClass {
        OpcodeClass::Sha256
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        todo!()
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        todo!()
    }
}
