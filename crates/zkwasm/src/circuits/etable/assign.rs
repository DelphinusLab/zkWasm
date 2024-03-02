use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::plonk::Error;
use log::debug;
use specs::configure_table::ConfigureTable;
use specs::itable::InstructionTable;
use specs::itable::OpcodeClassPlain;
use specs::state::InitializationState;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::EventTableChip;
use super::EventTableOpcodeConfig;
use super::EVENT_TABLE_ENTRY_ROWS;
use crate::circuits::cell::CellExpression;
use crate::circuits::utils::bn_to_field;
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

pub(in crate::circuits) struct EventTablePermutationCells<F: FieldExt> {
    pub(in crate::circuits) rest_mops: Option<AssignedCell<F, F>>,
    pub(in crate::circuits) rest_jops: Option<AssignedCell<F, F>>,
    pub(in crate::circuits) post_initialization_state: InitializationState<AssignedCell<F, F>>,
}

impl<F: FieldExt> EventTableChip<F> {
    fn assign_step_state(
        &self,
        ctx: &mut Context<'_, F>,
        state: &InitializationState<u32>,
        assigned_pre_initialization_state: Option<&InitializationState<AssignedCell<F, F>>>,
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
        let external_host_call_call_index = assign_common_range_advice!(
            external_host_call_index_cell,
            state.external_host_call_call_index
        );

        let initial_memory_pages =
            assign_common_range_advice!(mpages_cell, state.initial_memory_pages);
        let maximal_memory_pages =
            assign_common_range_advice!(maximal_memory_pages_cell, state.maximal_memory_pages);

        #[cfg(feature = "continuation")]
        let jops = assign_common_range_advice!(jops_cell, state.jops);

        if let Some(assigned_pre_initialization_state) = assigned_pre_initialization_state {
            macro_rules! constrain_equal {
                ($field:ident) => {
                    ctx.region.constrain_equal(
                        $field.cell(),
                        assigned_pre_initialization_state.$field.cell(),
                    )?;
                };
            }

            constrain_equal!(eid);
            constrain_equal!(fid);
            constrain_equal!(iid);
            constrain_equal!(sp);
            constrain_equal!(frame_id);

            constrain_equal!(host_public_inputs);
            constrain_equal!(context_in_index);
            constrain_equal!(context_out_index);
            constrain_equal!(external_host_call_call_index);

            constrain_equal!(initial_memory_pages);
            constrain_equal!(maximal_memory_pages);

            #[cfg(feature = "continuation")]
            constrain_equal!(jops);
        }

        // The context will be stepped by EVENT_TABLE_ENTRY_ROWS.
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
            external_host_call_call_index,

            initial_memory_pages,
            maximal_memory_pages,

            #[cfg(feature = "continuation")]
            jops,
        })
    }

    fn compute_rest_mops_and_jops(
        &self,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        _initialization_state: &InitializationState<u32>,
    ) -> (u32, u32) {
        let (rest_mops, _rest_jops) =
            event_table
                .0
                .iter()
                .fold((0, 0), |(rest_mops_sum, rest_jops_sum), entry| {
                    let instruction = entry.eentry.get_instruction(itable);

                    let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();

                    (
                        rest_mops_sum + op_config.memory_writing_ops(&entry.eentry),
                        rest_jops_sum + op_config.jops(),
                    )
                });

        cfg_if::cfg_if! {
            if #[cfg(feature="continuation")] {
                (rest_mops, _initialization_state.jops)
            } else {
                (rest_mops, _rest_jops)
            }
        }
    }

    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
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

        #[cfg(not(feature = "continuation"))]
        ctx.region.assign_advice_from_constant(
            || "etable: rest jops terminates",
            self.config.common_config.jops_cell.cell.col,
            ctx.offset,
            F::zero(),
        )?;

        Ok(())
    }

    // Get the cell to permutation, the meaningless value should be overwritten.
    fn assign_rest_ops_first_step(
        &self,
        ctx: &mut Context<'_, F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        let rest_mops_cell = self
            .config
            .common_config
            .rest_mops_cell
            .assign(ctx, F::zero())?;

        let rest_jops_cell = self.config.common_config.jops_cell.assign(ctx, F::zero())?;

        Ok((rest_mops_cell, rest_jops_cell))
    }

    fn assign_padding_and_post_initialization_state(
        &self,
        ctx: &mut Context<'_, F>,
        initialization_state: &InitializationState<u32>,
    ) -> Result<InitializationState<AssignedCell<F, F>>, Error> {
        while ctx.offset < self.capability * EVENT_TABLE_ENTRY_ROWS as usize {
            self.assign_step_state(ctx, initialization_state, None)?;
        }

        self.assign_step_state(ctx, initialization_state, None)
    }

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        initialization_state: &InitializationState<u32>,
        post_initialization_state: &InitializationState<u32>,
        mut rest_mops: u32,
        mut jops: u32,
    ) -> Result<(), Error> {
        macro_rules! assign_advice {
            ($cell:ident, $value:expr) => {
                self.config.common_config.$cell.assign(ctx, $value)?
            };
        }

        macro_rules! assign_advice_cell {
            ($cell:ident, $value:expr) => {
                $cell.assign(ctx, $value)?
            };
        }

        let mut host_public_inputs = initialization_state.host_public_inputs;
        let mut context_in_index = initialization_state.context_in_index;
        let mut context_out_index = initialization_state.context_out_index;
        let mut external_host_call_call_index = initialization_state.external_host_call_call_index;

        /*
         * Skip subsequent advice assignment in the first pass to enhance performance.
         */
        {
            let assigned_cell = assign_advice!(enabled_cell, F::zero());
            if assigned_cell.value().is_none() {
                return Ok(());
            }
        }

        /*
         * The length of event_table equals 0: without_witness
         */
        if event_table.0.len() == 0 {
            return Ok(());
        }

        let status = {
            let mut status = event_table
                .0
                .iter()
                .map(|entry| Status {
                    eid: entry.eentry.eid,
                    fid: entry.eentry.fid,
                    iid: entry.eentry.iid,
                    sp: entry.eentry.sp,
                    last_jump_eid: entry.eentry.last_jump_eid,
                    allocated_memory_pages: entry.eentry.allocated_memory_pages,
                    itable,
                })
                .collect::<Vec<_>>();

            let terminate_status = Status {
                itable,
                eid: post_initialization_state.eid,
                fid: post_initialization_state.fid,
                iid: post_initialization_state.iid,
                sp: post_initialization_state.sp,
                last_jump_eid: post_initialization_state.frame_id,
                allocated_memory_pages: post_initialization_state.initial_memory_pages,
            };

            status.push(terminate_status);

            status
        };

        for (index, entry) in event_table.0.iter().enumerate() {
            let instruction = entry.eentry.get_instruction(itable);

            let step_status = StepStatus {
                current: &status[index],
                next: &status[index + 1],
                current_external_host_call_index: external_host_call_call_index,
                configure_table,
                host_public_inputs,
                context_in_index,
                context_out_index,
            };

            {
                let class: OpcodeClassPlain = (&instruction.opcode).into();

                let op = self.config.common_config.ops[class.index()];
                assign_advice_cell!(op, F::one());
            }

            assign_advice!(enabled_cell, F::one());
            assign_advice!(rest_mops_cell, F::from(rest_mops as u64));
            assign_advice!(itable_lookup_cell, bn_to_field(&instruction.encode));
            assign_advice!(jops_cell, F::from(jops as u64));

            let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();
            op_config.assign(ctx, &step_status, &entry)?;

            // Be careful, the function will step context.
            self.assign_step_state(
                ctx,
                &InitializationState {
                    eid: entry.eentry.eid,
                    fid: entry.eentry.fid,
                    iid: entry.eentry.iid,
                    sp: entry.eentry.sp,
                    frame_id: entry.eentry.last_jump_eid,

                    host_public_inputs,
                    context_in_index,
                    context_out_index,
                    external_host_call_call_index,

                    initial_memory_pages: entry.eentry.allocated_memory_pages,
                    maximal_memory_pages: configure_table.maximal_memory_pages,

                    #[cfg(feature = "continuation")]
                    jops,
                },
                None,
            )?;

            if op_config.is_host_public_input(&entry.eentry) {
                host_public_inputs += 1;
            }
            if op_config.is_context_input_op(&entry.eentry) {
                context_in_index += 1;
            }
            if op_config.is_context_output_op(&entry.eentry) {
                context_out_index += 1;
            }
            if op_config.is_external_host_call(&entry.eentry) {
                external_host_call_call_index += 1;
            }

            rest_mops -= op_config.memory_writing_ops(&entry.eentry);
            if cfg!(feature = "continuation") {
                jops += op_config.jops()
            } else {
                jops -= op_config.jops()
            }
        }

        Ok(())
    }

    pub(in crate::circuits) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        initialization_state: &InitializationState<u32>,
        assigned_pre_initialization_state: &InitializationState<AssignedCell<F, F>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        post_initialization_state: &InitializationState<u32>,
        _is_last_slice: bool,
    ) -> Result<EventTablePermutationCells<F>, Error> {
        debug!("size of execution table: {}", event_table.0.len());

        assert!(event_table.0.len() <= self.capability);

        self.init(ctx)?;
        ctx.reset();

        self.assign_step_state(
            ctx,
            initialization_state,
            Some(assigned_pre_initialization_state),
        )?;
        ctx.reset();

        let (rest_mops_cell, _jops_cell) = self.assign_rest_ops_first_step(ctx)?;

        let (rest_mops, jops) = self.compute_rest_mops_and_jops(
            &self.config.op_configs,
            itable,
            event_table,
            initialization_state,
        );

        self.assign_entries(
            ctx,
            &self.config.op_configs,
            itable,
            event_table,
            configure_table,
            &initialization_state,
            post_initialization_state,
            rest_mops,
            jops,
        )?;

        let post_initialization_state_cells =
            self.assign_padding_and_post_initialization_state(ctx, &post_initialization_state)?;

        cfg_if::cfg_if! {
            if #[cfg(feature = "continuation")] {
                Ok(EventTablePermutationCells {
                    rest_mops: Some(rest_mops_cell),
                    rest_jops: if _is_last_slice {
                        Some(post_initialization_state_cells.jops.clone())
                    } else {
                        None
                    },
                    post_initialization_state: post_initialization_state_cells,
                })
            } else {
                Ok(EventTablePermutationCells {
                    rest_mops: Some(rest_mops_cell),
                    rest_jops: Some(_jops_cell),
                    post_initialization_state: post_initialization_state_cells,
                })
            }
        }
    }
}
