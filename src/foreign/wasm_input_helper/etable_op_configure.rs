use crate::{
    circuits::{
        etable_compact::{
            op_configure::{
                BitCell, CommonRangeCell, ConstraintBuilder, EventTableCellAllocator,
                EventTableOpcodeConfig, MTableLookupCell, U64Cell,
            },
            EventTableCommonConfig, MLookupItem, StepStatus,
        },
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant_from, constant_from_bn,
    foreign::{EventTableForeignCallConfigBuilder, ForeignCallInfo},
};
use halo2_proofs::{
    arithmetic::FieldExt,
    plonk::{Error, Expression, VirtualCells},
};
use num_bigint::BigUint;
use specs::step::StepInfo;
use specs::{
    etable::EventTableEntry,
    itable::{OpcodeClass, OPCODE_CLASS_SHIFT},
};
use specs::{host_function::HostPlugin, mtable::VarType};

use super::circuits::{InputTableEncode, WASM_INPUT_FOREIGN_TABLE_KEY};

pub struct WasmInputForeignCallInfo {}
impl ForeignCallInfo for WasmInputForeignCallInfo {
    fn call_id(&self) -> usize {
        OpcodeClass::ForeignPluginStart as usize + HostPlugin::HostInput as usize
    }
}

pub struct ETableWasmInputHelperTableConfig {
    foreign_call_id: u64,

    index: CommonRangeCell,
    public: BitCell,
    value: U64Cell,

    lookup_read_stack: MTableLookupCell,
    lookup_write_stack: MTableLookupCell,
}

pub struct ETableWasmInputHelperTableConfigBuilder {}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F>
    for ETableWasmInputHelperTableConfigBuilder
{
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let index = common.alloc_common_range_value();
        let public = common.alloc_bit_value();
        let value = common.alloc_u64();

        let lookup_read_stack = common.alloc_mtable_lookup();
        let lookup_write_stack = common.alloc_mtable_lookup();

        constraint_builder.lookup(
            WASM_INPUT_FOREIGN_TABLE_KEY,
            "lookup input table",
            Box::new(move |meta| {
                InputTableEncode::encode_for_lookup(index.expr(meta), value.expr(meta))
            }),
        );

        Box::new(ETableWasmInputHelperTableConfig {
            foreign_call_id: info.call_id() as u64,
            index,
            public,
            value,
            lookup_read_stack,
            lookup_write_stack,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETableWasmInputHelperTableConfig {
    fn opcode(&self, _meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        constant_from_bn!(&(BigUint::from(self.foreign_call_id) << OPCODE_CLASS_SHIFT))
    }

    fn opcode_class(&self) -> OpcodeClass {
        unreachable!()
    }

    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(2))
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I32),
                self.public.expr(meta),
            )),
            MLookupItem::Second => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                constant_from!(2),
                common_config.sp(meta) + constant_from!(1),
                constant_from!(VarType::I64),
                self.value.expr(meta),
            )),
            _ => None,
        }
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
                args,
                ret_val,
                signature,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::HostInput);
                assert_eq!(args.len(), 1);

                self.index.assign(ctx, 0)?;
                self.public.assign(ctx, (*args.get(0).unwrap()) == 1)?;
                self.value.assign(ctx, ret_val.unwrap())?;

                let arg_type: VarType = (*signature.params.get(0).unwrap()).into();
                let ret_type: VarType = signature.return_type.unwrap().into();

                self.lookup_read_stack.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_read(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(arg_type as u16),
                        BigUint::from(*args.get(0).unwrap()),
                    ),
                )?;

                self.lookup_write_stack.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(2 as u64),
                        BigUint::from(step_info.current.sp + 1),
                        BigUint::from(ret_type as u16),
                        BigUint::from(ret_val.unwrap()),
                    ),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }
}
