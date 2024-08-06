use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::circuit::Layouter;
use halo2_proofs::circuit::Region;
use halo2_proofs::plonk::Error;
use log::debug;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelSlice;
use specs::configure_table::ConfigureTable;
use specs::itable::InstructionTable;
use specs::itable::OpcodeClassPlain;
use specs::slice::FrameTableSlice;
use specs::state::InitializationState;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::EventTableChip;
use super::OpcodeConfig;
use super::EVENT_TABLE_ENTRY_ROWS;
use crate::circuits::cell::CellExpression;
use crate::circuits::jtable::FrameEtablePermutationCells;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::FieldHelper;
use crate::circuits::utils::step_status::Status;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::Context;

/*
 * Etable Layouter with Continuation
 *
 * Not last slice
 *   - `self.capability` entries with `enable = 1`.
 *   - one entry(only including status) with enable = 0, the status of this entry should constrain equality
 *     with the first entry in the next slice.
 *
 * | -------- | ---- | ------ | ---------- | ---- | ------ |
 * |          | sel  | enable | rest_mops  | jops | states |
 * | -------- | ---- + ------ | ---------- + ---- | ------ |
 * | event    |  1   |   1    |            |      |        | permutation with pre image table
 * | table    |  1   |   1    |            |      |        |
 * | entries  |  1   |   1    |            |      |        |
 * |          |  1   |   1    |            |      |        |
 * | -------- | ---- | ------ | ---------- | ---- | ------ |
 * |          |  0   |   0    | constant 0 |      |        | permutation with post image table
 *
 *
 * Last slice
 * ``
 * | -------- | ---- | ------ | ---------- | ---- | ------ |
 * |          | sel  | enable | rest_mops  | jops | states |
 * | -------- | ---- + ------ | ---------- + ---- | -------|
 * | event    |  1   |   1    |            |      |        | permutation with pre image table
 * | table    |  1   |   1    |            |      |        |
 * | entires  |  1   |   1    |            |      |        |
 * | -------- | ---- | ------ | ---------- | ---- | ------ |
 * | padding  |  1   |   0    |            |      |        | padding rows are used to copy termination status
 * |          |  1   |   0    |            |      |        | to permutation row
 * |          |  1   |   0    |            |      |        |
 * |          |  1   |   0    |            |      |        |
 * | -------- | ---- | ------ | ---------- | ---- | ------ |
 * |          |  0   |   0    | constant 0 |      |        | permutation with post image table/jops constrain with jtable
 */
#[derive(Debug)]
pub(in crate::circuits) struct EventTablePermutationCells<F: FieldExt> {
    pub(in crate::circuits) rest_mops: AssignedCell<F, F>,
    // rest_jops cell at first step
    pub(in crate::circuits) rest_jops: FrameEtablePermutationCells<F>,
    pub(in crate::circuits) pre_initialization_state: InitializationState<AssignedCell<F, F>>,
    pub(in crate::circuits) post_initialization_state: InitializationState<AssignedCell<F, F>>,
}

const EXTERNAL_HOST_CALL_START_INDEX: u32 = 1;

struct ExtraStatus {
    external_host_call_index: u32,
}

impl<F: FieldExt> EventTableChip<F> {
    fn assign_step_state<T: Borrow<ExtraStatus>>(
        &self,
        ctx: &mut Context<'_, F>,
        state: &InitializationState<u32>,
        extra_status: T,
    ) -> Result<InitializationState<AssignedCell<F, F>>, Error> {
        cfg_if::cfg_if! {
            if #[cfg(feature="continuation")] {
                macro_rules! assign_u32_state {
                    ($cell:ident, $value:expr) => {
                        self.config.common_config.$cell.assign(ctx, $value)?
                    };
                }
            } else {
                macro_rules! assign_u32_state {
                    ($cell:ident, $value:expr) => {
                        self.config.common_config.$cell.assign_u32(ctx, $value)?
                    };
                }
            }
        }

        macro_rules! assign_common_range_advice {
            ($cell:ident, $value:expr) => {
                self.config
                    .common_config
                    .$cell
                    .assign(ctx, F::from($value as u64))?
            };
        }

        let eid = assign_u32_state!(eid_cell, state.eid);
        let fid = assign_common_range_advice!(fid_cell, state.fid);
        let iid = assign_common_range_advice!(iid_cell, state.iid);
        let sp = assign_common_range_advice!(sp_cell, state.sp);
        let frame_id = assign_u32_state!(frame_id_cell, state.frame_id);

        let host_public_inputs =
            assign_common_range_advice!(input_index_cell, state.host_public_inputs);
        let context_in_index =
            assign_common_range_advice!(context_input_index_cell, state.context_in_index);
        let context_out_index =
            assign_common_range_advice!(context_output_index_cell, state.context_out_index);

        assign_common_range_advice!(
            external_host_call_index_cell,
            extra_status.borrow().external_host_call_index
        );

        let initial_memory_pages =
            assign_common_range_advice!(mpages_cell, state.initial_memory_pages);
        let maximal_memory_pages =
            assign_common_range_advice!(maximal_memory_pages_cell, state.maximal_memory_pages);

        ctx.step(EVENT_TABLE_ENTRY_ROWS as usize);

        Ok(InitializationState {
            eid,
            fid,
            iid,
            frame_id,
            sp,

            host_public_inputs,
            context_in_index,
            context_out_index,

            initial_memory_pages,
            maximal_memory_pages,
        })
    }

    fn compute_rest_mops_and_jops(
        &self,
        op_configs: Arc<BTreeMap<OpcodeClassPlain, OpcodeConfig<F>>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
    ) -> (u32, u32, u32) {
        let (rest_mops, rest_call_ops, rest_return_ops) = event_table.0.iter().fold(
            (0, 0, 0),
            |(rest_mops_sum, rest_call_ops_sum, rest_return_ops_sum), entry| {
                let instruction = entry.eentry.get_instruction(itable);

                let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();

                (
                    rest_mops_sum + op_config.0.memory_writing_ops(&entry.eentry),
                    rest_call_ops_sum + op_config.0.call_ops(),
                    rest_return_ops_sum + op_config.0.return_ops(),
                )
            },
        );

        (rest_mops, rest_call_ops, rest_return_ops)
    }

    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        assert_eq!(ctx.offset, 0);

        ctx.region.assign_advice_from_constant(
            || "etable: external host call index",
            self.config
                .common_config
                .external_host_call_index_cell
                .cell
                .col,
            ctx.offset,
            F::from(EXTERNAL_HOST_CALL_START_INDEX as u64),
        )?;

        for _ in 0..self.capability {
            ctx.region.assign_fixed(
                || "etable: step sel",
                self.config.step_sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            ctx.step(EVENT_TABLE_ENTRY_ROWS as usize);
        }

        ctx.region.assign_advice_from_constant(
            || "etable: rest mops terminates",
            self.config.common_config.rest_mops_cell.cell.col,
            ctx.offset,
            F::zero(),
        )?;

        ctx.region.assign_advice_from_constant(
            || "etable: rest call ops terminates",
            self.config.common_config.rest_call_ops_cell.cell.col,
            ctx.offset,
            F::zero(),
        )?;

        ctx.region.assign_advice_from_constant(
            || "etable: rest return ops terminates",
            self.config.common_config.rest_return_ops_cell.cell.col,
            ctx.offset,
            F::zero(),
        )?;

        Ok(())
    }

    // Get the cell to permutation, the meaningless value should be overwritten.
    fn assign_rest_ops_first_step(
        &self,
        ctx: &mut Context<'_, F>,
    ) -> Result<(AssignedCell<F, F>, FrameEtablePermutationCells<F>), Error> {
        let rest_mops_cell = self
            .config
            .common_config
            .rest_mops_cell
            .assign(ctx, F::zero())?;

        let rest_call_ops_cell = self
            .config
            .common_config
            .rest_call_ops_cell
            .assign(ctx, F::zero())?;
        let rest_return_ops_cell = self
            .config
            .common_config
            .rest_return_ops_cell
            .assign(ctx, F::zero())?;

        Ok((
            rest_mops_cell,
            FrameEtablePermutationCells {
                rest_call_ops: rest_call_ops_cell,
                rest_return_ops: rest_return_ops_cell,
            },
        ))
    }

    fn assign_padding_and_post_initialization_state(
        &self,
        ctx: &mut Context<'_, F>,
        initialization_state: &InitializationState<u32>,
        extra_status: &ExtraStatus,
    ) -> Result<InitializationState<AssignedCell<F, F>>, Error> {
        while ctx.offset < self.capability * EVENT_TABLE_ENTRY_ROWS as usize {
            self.assign_step_state(ctx, initialization_state, extra_status)?;
        }

        self.assign_step_state(ctx, initialization_state, extra_status)
    }

    fn assign_entries(
        &self,
        region: &Region<'_, F>,
        op_configs: Arc<BTreeMap<OpcodeClassPlain, OpcodeConfig<F>>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        frame_table: &FrameTableSlice,
        initialization_state: &InitializationState<u32>,
        post_initialization_state: &InitializationState<u32>,
        rest_mops: u32,
        rest_call_ops: u32,
        rest_return_ops: u32,
    ) -> Result<ExtraStatus, Error> {
        macro_rules! assign_advice {
            ($ctx:expr, $cell:ident, $value:expr) => {
                self.config
                    .common_config
                    .$cell
                    .assign($ctx, $value)
                    .unwrap()
            };
        }

        macro_rules! assign_advice_cell {
            ($ctx:expr, $cell:ident, $value:expr) => {
                $cell.assign($ctx, $value).unwrap()
            };
        }

        let frame_table_returned_lookup = frame_table.build_returned_lookup_mapping();

        let status = {
            let mut host_public_inputs = initialization_state.host_public_inputs;
            let mut context_in_index = initialization_state.context_in_index;
            let mut context_out_index = initialization_state.context_out_index;
            let mut external_host_call_call_index = EXTERNAL_HOST_CALL_START_INDEX;

            let mut rest_mops = rest_mops;
            let mut rest_call_ops = rest_call_ops;
            let mut rest_return_ops = rest_return_ops;

            let mut status = event_table
                .0
                .iter()
                .map(|entry| {
                    let op_config = op_configs
                        .get(&((&entry.eentry.get_instruction(itable).opcode).into()))
                        .unwrap();

                    let status = Status {
                        eid: entry.eentry.eid,
                        fid: entry.eentry.fid,
                        iid: entry.eentry.iid,
                        sp: entry.eentry.sp,
                        last_jump_eid: entry.eentry.last_jump_eid,
                        allocated_memory_pages: entry.eentry.allocated_memory_pages,

                        rest_mops,
                        rest_call_ops,
                        rest_return_ops,

                        host_public_inputs,
                        context_in_index,
                        context_out_index,
                        external_host_call_call_index,

                        itable,
                    };

                    if op_config.0.is_host_public_input(&entry.eentry) {
                        host_public_inputs += 1;
                    }
                    if op_config.0.is_context_input_op(&entry.eentry) {
                        context_in_index += 1;
                    }
                    if op_config.0.is_context_output_op(&entry.eentry) {
                        context_out_index += 1;
                    }
                    if op_config.0.is_external_host_call(&entry.eentry) {
                        external_host_call_call_index += 1;
                    }

                    rest_mops -= op_config.0.memory_writing_ops(&entry.eentry);
                    rest_call_ops -= op_config.0.call_ops();
                    rest_return_ops -= op_config.0.return_ops();

                    status
                })
                .collect::<Vec<_>>();

            assert_eq!(
                post_initialization_state.host_public_inputs,
                host_public_inputs
            );
            assert_eq!(post_initialization_state.context_in_index, context_in_index);
            assert_eq!(
                post_initialization_state.context_out_index,
                context_out_index
            );

            let terminate_status = Status {
                eid: post_initialization_state.eid,
                fid: post_initialization_state.fid,
                iid: post_initialization_state.iid,
                sp: post_initialization_state.sp,
                last_jump_eid: post_initialization_state.frame_id,
                allocated_memory_pages: post_initialization_state.initial_memory_pages,

                host_public_inputs: post_initialization_state.host_public_inputs,
                context_in_index: post_initialization_state.context_in_index,
                context_out_index: post_initialization_state.context_out_index,
                external_host_call_call_index,

                rest_mops,
                rest_call_ops,
                rest_return_ops,

                itable,
            };

            status.push(terminate_status);

            status
        };

        const THREAD: usize = 16;
        let chunk_size = if event_table.0.is_empty() {
            1
        } else {
            (event_table.0.len() + THREAD - 1) / THREAD
        };

        event_table
            .0
            .par_chunks(chunk_size)
            .enumerate()
            .for_each(|(chunk_index, entries)| {
                let mut ctx = Context::new(region);
                ctx.offset = (chunk_size * chunk_index) * (EVENT_TABLE_ENTRY_ROWS as usize);

                let mut field_helper = FieldHelper::default();

                for (index, entry) in entries.iter().enumerate() {
                    let index = chunk_index * chunk_size + index;

                    let instruction = entry.eentry.get_instruction(itable);

                    let mut step_status = StepStatus {
                        current: &status[index],
                        next: &status[index + 1],
                        configure_table,
                        frame_table_returned_lookup: &frame_table_returned_lookup,
                        field_helper: &mut field_helper,
                    };

                    {
                        let class: OpcodeClassPlain = (&instruction.opcode).into();

                        let op = self.config.common_config.ops[class.index()];
                        assign_advice_cell!(&mut ctx, op, F::one());
                    }

                    assign_advice!(&mut ctx, enabled_cell, F::one());
                    assign_advice!(
                        &mut ctx,
                        rest_mops_cell,
                        F::from(status[index].rest_mops as u64)
                    );
                    assign_advice!(
                        &mut ctx,
                        itable_lookup_cell,
                        bn_to_field(&instruction.encode)
                    );
                    assign_advice!(
                        &mut ctx,
                        rest_call_ops_cell,
                        F::from(status[index].rest_call_ops as u64)
                    );
                    assign_advice!(
                        &mut ctx,
                        rest_return_ops_cell,
                        F::from(status[index].rest_return_ops as u64)
                    );

                    {
                        let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();
                        op_config
                            .0
                            .assign(&mut ctx, &mut step_status, entry)
                            .unwrap();
                    }

                    // Be careful, the function will step context.
                    self.assign_step_state(
                        &mut ctx,
                        &InitializationState {
                            eid: entry.eentry.eid,
                            fid: entry.eentry.fid,
                            iid: entry.eentry.iid,
                            sp: entry.eentry.sp,
                            frame_id: entry.eentry.last_jump_eid,

                            host_public_inputs: status[index].host_public_inputs,
                            context_in_index: status[index].context_in_index,
                            context_out_index: status[index].context_out_index,

                            initial_memory_pages: entry.eentry.allocated_memory_pages,
                            maximal_memory_pages: configure_table.maximal_memory_pages,
                        },
                        ExtraStatus {
                            external_host_call_index: status[index].external_host_call_call_index,
                        },
                    )
                    .unwrap();
                }
            });

        Ok(ExtraStatus {
            external_host_call_index: status.last().unwrap().external_host_call_call_index,
        })
    }

    pub(in crate::circuits) fn assign(
        &self,
        layouter: impl Layouter<F>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        frame_table: &FrameTableSlice,
        initialization_state: &InitializationState<u32>,
        post_initialization_state: &InitializationState<u32>,
        _is_last_slice: bool,
    ) -> Result<EventTablePermutationCells<F>, Error> {
        layouter.assign_region(
            || "event table",
            |region| {
                let mut ctx = Context::new(region);

                debug!("size of execution table: {}", event_table.0.len());

                assert!(event_table.0.len() <= self.capability);

                self.init(&mut ctx)?;
                ctx.reset();

                let pre_initialization_state = self.assign_step_state(
                    &mut ctx,
                    initialization_state,
                    ExtraStatus {
                        external_host_call_index: EXTERNAL_HOST_CALL_START_INDEX,
                    },
                )?;
                ctx.reset();

                let (rest_mops_cell, rest_frame_table_cells) =
                    self.assign_rest_ops_first_step(&mut ctx)?;

                let (rest_mops, rest_call_ops, rest_return_ops) = self.compute_rest_mops_and_jops(
                    self.config.op_configs.clone(),
                    itable,
                    event_table,
                );

                let termination_status = self.assign_entries(
                    region,
                    self.config.op_configs.clone(),
                    itable,
                    event_table,
                    configure_table,
                    frame_table,
                    initialization_state,
                    post_initialization_state,
                    rest_mops,
                    rest_call_ops,
                    rest_return_ops,
                )?;
                ctx.step(EVENT_TABLE_ENTRY_ROWS as usize * event_table.0.len());

                let post_initialization_state_cells = self
                    .assign_padding_and_post_initialization_state(
                        &mut ctx,
                        post_initialization_state,
                        &termination_status,
                    )?;

                Ok(EventTablePermutationCells {
                    rest_mops: rest_mops_cell,
                    rest_jops: rest_frame_table_cells,
                    pre_initialization_state,
                    post_initialization_state: post_initialization_state_cells,
                })
            },
        )
    }
}
