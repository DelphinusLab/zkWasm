use super::{
    circuits::Sha2HelperEncode, Sha256HelperOp, SHA256_FOREIGN_FUNCTION_NAME_CH,
    SHA256_FOREIGN_FUNCTION_NAME_MAJ, SHA256_FOREIGN_TABLE_KEY,
};
use crate::{
    circuits::{
        etable_compact::{
            op_configure::{
                BitCell, ConstraintBuilder, EventTableCellAllocator, EventTableOpcodeConfig,
                MTableLookupCell, U64OnU8Cell,
            },
            EventTableCommonConfig, MLookupItem, StepStatus,
        },
        mtable_compact::encode::MemoryTableLookupEncode,
        utils::{bn_to_field, Context},
    },
    constant_from, constant_from_bn,
    foreign::{
        sha256_helper::{
            SHA256_FOREIGN_FUNCTION_NAME_LSIGMA0, SHA256_FOREIGN_FUNCTION_NAME_LSIGMA1,
            SHA256_FOREIGN_FUNCTION_NAME_SSIGMA0, SHA256_FOREIGN_FUNCTION_NAME_SSIGMA1,
        },
        EventTableForeignCallConfigBuilder, ForeignCallInfo,
    },
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

pub struct ETableSha256HelperTableConfig {
    foreign_call_id: u64,

    a: U64OnU8Cell,
    b: U64OnU8Cell,
    c: U64OnU8Cell,
    res: U64OnU8Cell,
    is_ssignma0: BitCell,
    is_ssignma1: BitCell,
    is_lsignma0: BitCell,
    is_lsignma1: BitCell,
    is_ch: BitCell,
    is_maj: BitCell,

    lookup_stack_read_a: MTableLookupCell,
    lookup_stack_read_b: MTableLookupCell,
    lookup_stack_read_c: MTableLookupCell,
    lookup_stack_write: MTableLookupCell,
}

pub struct Sha256ForeignCallInfo {}
impl ForeignCallInfo for Sha256ForeignCallInfo {
    fn call_id(&self) -> usize {
        OpcodeClass::ForeignPluginStart as usize + HostPlugin::Sha256 as usize
    }
}
pub struct ETableSha256HelperTableConfigBuilder {}

impl<F: FieldExt> EventTableForeignCallConfigBuilder<F> for ETableSha256HelperTableConfigBuilder {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
        info: &impl ForeignCallInfo,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let a = common.alloc_u64_on_u8();
        let b = common.alloc_u64_on_u8();
        let c = common.alloc_u64_on_u8();
        let res = common.alloc_u64_on_u8();

        let is_ssignma0 = common.alloc_bit_value();
        let is_ssignma1 = common.alloc_bit_value();
        let is_lsignma0 = common.alloc_bit_value();
        let is_lsignma1 = common.alloc_bit_value();
        let is_ch = common.alloc_bit_value();
        let is_maj = common.alloc_bit_value();

        let lookup_stack_read_a = common.alloc_mtable_lookup();
        let lookup_stack_read_b = common.alloc_mtable_lookup();
        let lookup_stack_read_c = common.alloc_mtable_lookup();
        let lookup_stack_write = common.alloc_mtable_lookup();

        constraint_builder.push(
            "sha256helper: is one of ops",
            Box::new(move |meta| {
                vec![
                    (is_ssignma0.expr(meta)
                        + is_ssignma1.expr(meta)
                        + is_lsignma0.expr(meta)
                        + is_lsignma1.expr(meta)
                        + is_ch.expr(meta)
                        + is_maj.expr(meta)
                        - constant_from!(1)),
                ]
            }),
        );

        constraint_builder.lookup(
            SHA256_FOREIGN_TABLE_KEY,
            "sha256 helper table lookup",
            Box::new(move |meta| {
                let op = is_ssignma0.expr(meta) * constant_from!(Sha256HelperOp::SSigma0)
                    + is_ssignma1.expr(meta) * constant_from!(Sha256HelperOp::SSigma1)
                    + is_lsignma0.expr(meta) * constant_from!(Sha256HelperOp::LSigma0)
                    + is_lsignma1.expr(meta) * constant_from!(Sha256HelperOp::LSigma1)
                    + is_ch.expr(meta) * constant_from!(Sha256HelperOp::Ch)
                    + is_maj.expr(meta) * constant_from!(Sha256HelperOp::Maj);
                Sha2HelperEncode::encode_opcode_expr(
                    op,
                    vec![a.expr(meta), b.expr(meta), c.expr(meta)],
                    res.expr(meta),
                )
            }),
        );

        Box::new(ETableSha256HelperTableConfig {
            foreign_call_id: info.call_id() as u64,
            a,
            b,
            c,
            res,
            is_ssignma0,
            is_ssignma1,
            is_lsignma0,
            is_lsignma1,
            is_ch,
            is_maj,
            lookup_stack_read_a,
            lookup_stack_read_b,
            lookup_stack_read_c,
            lookup_stack_write,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for ETableSha256HelperTableConfig {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let pick_one = self.is_ssignma0.expr(meta) * constant_from!(Sha256HelperOp::SSigma0)
            + self.is_ssignma1.expr(meta) * constant_from!(Sha256HelperOp::SSigma1)
            + self.is_lsignma0.expr(meta) * constant_from!(Sha256HelperOp::LSigma0)
            + self.is_lsignma1.expr(meta) * constant_from!(Sha256HelperOp::LSigma1)
            + self.is_ch.expr(meta) * constant_from!(Sha256HelperOp::Ch)
            + self.is_maj.expr(meta) * constant_from!(Sha256HelperOp::Maj);

        constant_from_bn!(&(BigUint::from(self.foreign_call_id) << OPCODE_CLASS_SHIFT)) + pick_one
    }

    fn mops(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        let is_four_mops = self.is_ch.expr(meta) + self.is_maj.expr(meta);
        Some(constant_from!(2) * is_four_mops + constant_from!(2))
    }

    fn assigned_extra_mops(
        &self,
        _ctx: &mut Context<'_, F>,
        _step: &StepStatus,
        entry: &EventTableEntry,
    ) -> u64 {
        match &entry.step_info {
            StepInfo::CallHost { function_name, .. } => {
                if function_name == SHA256_FOREIGN_FUNCTION_NAME_CH
                    || function_name == SHA256_FOREIGN_FUNCTION_NAME_MAJ
                {
                    4
                } else {
                    2
                }
            }
            _ => unreachable!(),
        }
    }

    fn mtable_lookup(
        &self,
        meta: &mut VirtualCells<'_, F>,
        item: MLookupItem,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        let is_four_mops = self.is_ch.expr(meta) + self.is_maj.expr(meta);
        match item {
            MLookupItem::First => Some(MemoryTableLookupEncode::encode_stack_read(
                common_config.eid(meta),
                constant_from!(1),
                common_config.sp(meta)
                    + constant_from!(1)
                    + is_four_mops.clone() * constant_from!(2),
                constant_from!(VarType::I32),
                self.a.expr(meta),
            )),
            MLookupItem::Second => Some(
                is_four_mops.clone()
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(2),
                        common_config.sp(meta) + constant_from!(2),
                        constant_from!(VarType::I32),
                        self.b.expr(meta),
                    ),
            ),
            MLookupItem::Third => Some(
                is_four_mops.clone()
                    * MemoryTableLookupEncode::encode_stack_read(
                        common_config.eid(meta),
                        constant_from!(3),
                        common_config.sp(meta) + constant_from!(1),
                        constant_from!(VarType::I32),
                        self.c.expr(meta),
                    ),
            ),
            MLookupItem::Fourth => Some(MemoryTableLookupEncode::encode_stack_write(
                common_config.eid(meta),
                is_four_mops.clone() * constant_from!(2) + constant_from!(2),
                common_config.sp(meta) + constant_from!(1) + is_four_mops * constant_from!(2),
                constant_from!(VarType::I32),
                self.res.expr(meta),
            )),
            _ => None,
        }
    }

    fn sp_diff(&self, meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        let is_four_mops = self.is_ch.expr(meta) + self.is_maj.expr(meta);
        Some(constant_from!(2) * is_four_mops)
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
                function_name,
                args,
                ret_val,
                ..
            } => {
                assert_eq!(*plugin, HostPlugin::Sha256);

                for (arg, v) in vec![&self.a, &self.b, &self.c].into_iter().zip(args.iter()) {
                    arg.assign(ctx, *v)?;
                }

                self.res.assign(ctx, ret_val.unwrap())?;

                if function_name == SHA256_FOREIGN_FUNCTION_NAME_MAJ {
                    self.is_maj.assign(ctx, true)?;
                }
                if function_name == SHA256_FOREIGN_FUNCTION_NAME_CH {
                    self.is_ch.assign(ctx, true)?;
                }
                if function_name == SHA256_FOREIGN_FUNCTION_NAME_SSIGMA0 {
                    self.is_ssignma0.assign(ctx, true)?;
                }
                if function_name == SHA256_FOREIGN_FUNCTION_NAME_SSIGMA1 {
                    self.is_ssignma1.assign(ctx, true)?;
                }
                if function_name == SHA256_FOREIGN_FUNCTION_NAME_LSIGMA0 {
                    self.is_lsignma0.assign(ctx, true)?;
                }
                if function_name == SHA256_FOREIGN_FUNCTION_NAME_LSIGMA1 {
                    self.is_lsignma1.assign(ctx, true)?;
                }

                for (i, (lookup, v)) in vec![
                    &self.lookup_stack_read_a,
                    &self.lookup_stack_read_b,
                    &self.lookup_stack_read_c,
                ]
                .into_iter()
                .zip(args.iter())
                .enumerate()
                {
                    lookup.assign(
                        ctx,
                        &MemoryTableLookupEncode::encode_stack_read(
                            BigUint::from(step_info.current.eid),
                            BigUint::from(1 + i as u64),
                            BigUint::from(step_info.current.sp + args.len() as u32 - i as u32),
                            BigUint::from(VarType::I32 as u64),
                            BigUint::from(*v),
                        ),
                    )?;
                }

                self.lookup_stack_write.assign(
                    ctx,
                    &MemoryTableLookupEncode::encode_stack_write(
                        BigUint::from(step_info.current.eid),
                        BigUint::from(1 + args.len() as u64),
                        BigUint::from(step_info.current.sp + args.len() as u32),
                        BigUint::from(VarType::I32 as u64),
                        BigUint::from(ret_val.unwrap()),
                    ),
                )?;
            }
            _ => unreachable!(),
        };
        Ok(())
    }
}
