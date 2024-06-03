use crate::circuits::cell::*;
use crate::circuits::etable::allocator::*;
use crate::circuits::etable::ConstraintBuilder;
use crate::circuits::etable::EventTableCommonConfig;
use crate::circuits::etable::EventTableOpcodeConfig;
use crate::circuits::etable::EventTableOpcodeConfigBuilder;
use crate::circuits::jtable::expression::JtableLookupEntryEncode;
use crate::circuits::jtable::JumpTableConfig;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::Context;
use crate::constant_from;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use specs::encode::frame_table::encode_frame_table_entry;
use specs::encode::opcode::encode_call;
use specs::step::StepInfo;

pub struct CallConfig<F: FieldExt> {
    // indicates if the calling returned in current slice.
    is_returned_cell: AllocatedBitCell<F>,
    index_cell: AllocatedCommonRangeCell<F>,
    frame_table_lookup: AllocatedUnlimitedCell<F>,
}

pub struct CallConfigBuilder {}

impl<F: FieldExt> EventTableOpcodeConfigBuilder<F> for CallConfigBuilder {
    fn configure(
        common_config: &EventTableCommonConfig<F>,
        allocator: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>> {
        let index_cell = allocator.alloc_common_range_cell();
        let frame_table_lookup = common_config.jtable_lookup_cell;

        let fid_cell = common_config.fid_cell;
        let iid_cell = common_config.iid_cell;
        let frame_id_cell = common_config.frame_id_cell;
        let eid = common_config.eid_cell;

        constraint_builder.constraints.push((
            "return frame table lookups",
            Box::new(move |meta| {
                vec![
                    frame_table_lookup.cell.expr(meta)
                        - JumpTableConfig::encode_lookup(
                            eid.expr(meta),
                            frame_id_cell.expr(meta),
                            index_cell.expr(meta),
                            fid_cell.expr(meta),
                            iid_cell.expr(meta) + constant_from!(1),
                        ),
                ]
            }),
        ));

        Box::new(CallConfig {
            is_returned_cell: common_config.is_returned_cell,
            index_cell,
            frame_table_lookup,
        })
    }
}

impl<F: FieldExt> EventTableOpcodeConfig<F> for CallConfig<F> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        encode_call(self.index_cell.expr(meta))
    }

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &mut StepStatus<F>,
        entry: &EventTableEntryWithMemoryInfo,
    ) -> Result<(), Error> {
        match &entry.eentry.step_info {
            StepInfo::Call { index } => {
                self.index_cell.assign(ctx, F::from(*index as u64))?;
                self.frame_table_lookup.cell.assign(
                    ctx,
                    bn_to_field(&encode_frame_table_entry(
                        step.current.eid.into(),
                        step.current.last_jump_eid.into(),
                        (*index).into(),
                        step.current.fid.into(),
                        (step.current.iid + 1).into(),
                    )),
                )?;
                self.is_returned_cell.assign(
                    ctx,
                    (*step
                        .frame_table_returned_lookup
                        .get(&(step.current.eid, *index))
                        .unwrap())
                    .into(),
                )?;

                Ok(())
            }

            _ => unreachable!(),
        }
    }

    fn call_ops_expr(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        Some(constant_from!(self.call_ops() as u64))
    }

    fn call_ops(&self) -> u32 {
        1
    }

    fn next_frame_id(
        &self,
        meta: &mut VirtualCells<'_, F>,
        common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(common_config.eid_cell.curr_expr(meta))
    }

    fn next_fid(
        &self,
        meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(self.index_cell.expr(meta))
    }

    fn next_iid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        Some(constant_from!(0))
    }
}
