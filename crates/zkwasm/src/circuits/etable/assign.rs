use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Error;
use log::debug;
use specs::configure_table::ConfigureTable;
use specs::itable::Opcode;
use specs::itable::OpcodeClassPlain;
use specs::InitializationState;
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

pub(in crate::circuits) struct EventTablePermutationCells {
    pub(in crate::circuits) initialization_state: InitializationState<Cell>,
    pub(in crate::circuits) rest_jops: Cell,
}

impl<F: FieldExt> EventTableChip<F> {
    fn compute_rest_mops_and_jops(
        &self,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        event_table: &EventTableWithMemoryInfo,
    ) -> Vec<(u32, u32)> {
        let mut rest_ops = vec![];

        event_table
            .0
            .iter()
            .rev()
            .fold((0, 0), |(rest_mops_sum, rest_jops_sum), entry| {
                let op_config = op_configs
                    .get(&entry.eentry.inst.opcode.clone().into())
                    .unwrap();

                let acc = (
                    rest_mops_sum + op_config.memory_writing_ops(&entry.eentry),
                    rest_jops_sum + op_config.jops(),
                );

                rest_ops.push(acc);

                acc
            });

        rest_ops.reverse();

        rest_ops
    }

    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        let capability = self.max_available_rows / EVENT_TABLE_ENTRY_ROWS as usize;

        for _ in 0..capability {
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
            self.config.common_config.rest_mops_cell.0.col,
            ctx.offset,
            F::zero(),
        )?;

        ctx.region.assign_advice_from_constant(
            || "etable: rest jops terminates",
            self.config.common_config.rest_jops_cell.0.col,
            ctx.offset,
            F::zero(),
        )?;

        Ok(())
    }

    fn assign_rest_ops_first_step(
        &self,
        ctx: &mut Context<'_, F>,
        rest_mops: u32,
        rest_jops: u32,
    ) -> Result<(Cell, Cell), Error> {
        let rest_mops_cell = self
            .config
            .common_config
            .rest_mops_cell
            .assign(ctx, F::from(rest_mops as u64))?;

        let rest_mops_jell = self
            .config
            .common_config
            .rest_jops_cell
            .assign(ctx, F::from(rest_jops as u64))?;

        Ok((rest_mops_cell.cell(), rest_mops_jell.cell()))
    }

    fn assign_initialization_state(
        &self,
        ctx: &mut Context<'_, F>,
        configure_table: &ConfigureTable,
        initialization_state: &InitializationState<u32>,
    ) -> Result<InitializationState<Cell>, Error> {
        #[cfg(feature = "continuation")]
        macro_rules! assign_state_advice {
            ($cell:ident, $value:expr) => {
                self.config.common_config.$cell.assign(ctx, $value)?.cell()
            };
        }

        #[cfg(not(feature = "continuation"))]
        macro_rules! assign_state_advice {
            ($cell:ident, $value:expr) => {
                assign_common_range_advice!($cell, $value)
            };
        }

        macro_rules! assign_common_range_advice {
            ($cell:ident, $value:expr) => {
                self.config
                    .common_config
                    .$cell
                    .assign(ctx, F::from($value as u64))?
                    .cell()
            };
        }

        let eid = assign_state_advice!(eid_cell, initialization_state.eid);
        let fid = assign_common_range_advice!(fid_cell, initialization_state.fid);
        let iid = assign_common_range_advice!(iid_cell, initialization_state.iid);
        let sp = assign_common_range_advice!(sp_cell, initialization_state.sp);
        let frame_id = assign_state_advice!(frame_id_cell, initialization_state.frame_id);

        let host_public_inputs =
            assign_common_range_advice!(input_index_cell, initialization_state.host_public_inputs);
        let context_in_index = assign_common_range_advice!(
            context_input_index_cell,
            initialization_state.context_in_index
        );
        let context_out_index = assign_common_range_advice!(
            context_output_index_cell,
            initialization_state.context_out_index
        );
        let external_host_call_call_index = assign_common_range_advice!(
            external_host_call_index_cell,
            initialization_state.external_host_call_call_index
        );

        let initial_memory_pages =
            assign_common_range_advice!(mpages_cell, initialization_state.initial_memory_pages);
        let maximal_memory_pages = assign_common_range_advice!(
            maximal_memory_pages_cell,
            configure_table.maximal_memory_pages
        );

        #[cfg(feature = "continuation")]
        let jops = assign_common_range_advice!(jops_cell, initialization_state.jops);

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

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        rest_ops: Vec<(u32, u32)>,
        initialization_state: &InitializationState<u32>,
    ) -> Result<(), Error> {
        #[cfg(feature = "continuation")]
        macro_rules! assign_state_advice {
            ($cell:ident, $value:expr) => {
                self.config.common_config.$cell.assign(ctx, $value)?.cell()
            };
        }

        #[cfg(not(feature = "continuation"))]
        macro_rules! assign_state_advice {
            ($cell:ident, $value:expr) => {
                self.config
                    .common_config
                    .$cell
                    .assign(ctx, F::from($value as u64))?
                    .cell()
            };
        }

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

        let mut host_public_inputs = initialization_state.host_public_inputs;
        let mut context_in_index = initialization_state.context_in_index;
        let mut context_out_index = initialization_state.context_out_index;
        let mut external_host_call_call_index = initialization_state.external_host_call_call_index;

        let status = {
            let mut status = event_table
                .0
                .iter()
                .map(|entry| Status {
                    eid: entry.eentry.eid,
                    fid: entry.eentry.inst.fid,
                    iid: entry.eentry.inst.iid,
                    sp: entry.eentry.sp,
                    last_jump_eid: entry.eentry.last_jump_eid,
                    allocated_memory_pages: entry.eentry.allocated_memory_pages,
                })
                .collect::<Vec<_>>();

            let terminate_status = Status {
                eid: status.last().unwrap().eid + 1,
                fid: 0,
                iid: 0,
                sp: status.last().unwrap().sp
                    + if let Opcode::Return { drop, .. } =
                        &event_table.0.last().unwrap().eentry.inst.opcode
                    {
                        *drop
                    } else {
                        unreachable!()
                    },
                last_jump_eid: 0,
                allocated_memory_pages: status.last().unwrap().allocated_memory_pages,
            };

            status.push(terminate_status);

            status
        };

        for (index, (entry, (rest_mops, rest_jops))) in
            event_table.0.iter().zip(rest_ops.iter()).enumerate()
        {
            let step_status = StepStatus {
                current: &status[index],
                next: &status[index + 1],
                current_external_host_call_index: external_host_call_call_index,
                maximal_memory_pages: configure_table.maximal_memory_pages,
                host_public_inputs,
                context_in_index,
                context_out_index,
            };

            {
                let class: OpcodeClassPlain = entry.eentry.inst.opcode.clone().into();

                let op = self.config.common_config.ops[class.index()];
                assign_advice_cell!(op, F::one());
            }

            assign_advice!(enabled_cell, F::one());
            assign_advice!(rest_mops_cell, F::from(*rest_mops as u64));
            assign_advice!(rest_jops_cell, F::from(*rest_jops as u64));
            assign_advice!(input_index_cell, F::from(host_public_inputs as u64));
            assign_advice!(context_input_index_cell, F::from(context_in_index as u64));
            assign_advice!(context_output_index_cell, F::from(context_out_index as u64));
            assign_advice!(
                external_host_call_index_cell,
                F::from(external_host_call_call_index as u64)
            );
            assign_advice!(sp_cell, F::from(entry.eentry.sp as u64));
            assign_advice!(
                mpages_cell,
                F::from(entry.eentry.allocated_memory_pages as u64)
            );
            assign_advice!(
                maximal_memory_pages_cell,
                F::from(configure_table.maximal_memory_pages as u64)
            );
            assign_state_advice!(frame_id_cell, entry.eentry.last_jump_eid);
            assign_state_advice!(eid_cell, entry.eentry.eid);
            assign_advice!(fid_cell, F::from(entry.eentry.inst.fid as u64));
            assign_advice!(iid_cell, F::from(entry.eentry.inst.iid as u64));
            assign_advice!(itable_lookup_cell, bn_to_field(&entry.eentry.inst.encode()));

            let op_config = op_configs
                .get(&entry.eentry.inst.opcode.clone().into())
                .unwrap();
            op_config.assign(ctx, &step_status, &entry)?;

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

            ctx.step(EVENT_TABLE_ENTRY_ROWS as usize);
        }

        // Assign terminate status
        assign_state_advice!(eid_cell, status.last().unwrap().eid);
        assign_advice!(fid_cell, F::from(status.last().unwrap().fid as u64));
        assign_advice!(iid_cell, F::from(status.last().unwrap().iid as u64));
        assign_advice!(sp_cell, F::from(status.last().unwrap().sp as u64));
        assign_state_advice!(frame_id_cell, status.last().unwrap().last_jump_eid);
        assign_advice!(
            mpages_cell,
            F::from(status.last().unwrap().allocated_memory_pages as u64)
        );
        assign_advice!(
            maximal_memory_pages_cell,
            F::from(configure_table.maximal_memory_pages as u64)
        );
        assign_advice!(input_index_cell, F::from(host_public_inputs as u64));
        assign_advice!(context_input_index_cell, F::from(context_in_index as u64));
        assign_advice!(context_output_index_cell, F::from(context_out_index as u64));
        assign_advice!(
            external_host_call_index_cell,
            F::from(external_host_call_call_index as u64)
        );

        Ok(())
    }

    pub(in crate::circuits) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        initialization_state: &InitializationState<u32>,
    ) -> Result<EventTablePermutationCells, Error> {
        debug!("size of execution table: {}", event_table.0.len());
        assert!(event_table.0.len() * EVENT_TABLE_ENTRY_ROWS as usize <= self.max_available_rows);

        let rest_ops = self.compute_rest_mops_and_jops(&self.config.op_configs, event_table);

        self.init(ctx)?;
        ctx.reset();

        let (rest_mops_cell, rest_jops_cell) = self.assign_rest_ops_first_step(
            ctx,
            rest_ops.first().map_or(0u32, |(rest_mops, _)| *rest_mops),
            rest_ops.first().map_or(0u32, |(_, rest_jops)| *rest_jops),
        )?;
        ctx.reset();

        let initialization_state_cells =
            self.assign_initialization_state(ctx, configure_table, initialization_state)?;
        ctx.reset();

        self.assign_entries(
            ctx,
            &self.config.op_configs,
            event_table,
            configure_table,
            rest_ops,
            &initialization_state,
        )?;
        ctx.reset();

        Ok(EventTablePermutationCells {
            initialization_state: initialization_state_cells,
            rest_jops: rest_jops_cell,
        })
    }
}
