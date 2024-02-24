use ark_std::end_timer;
use ark_std::start_timer;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::circuit::Cell;
use halo2_proofs::plonk::Error;
use log::debug;
use specs::configure_table::ConfigureTable;
use specs::itable::InstructionTable;
use specs::itable::OpcodeClassPlain;
use specs::step::StepInfo;
use std::collections::BTreeMap;
use wasmi::DEFAULT_VALUE_STACK_LIMIT;

use super::EventTableChip;
use super::EventTableOpcodeConfigBox;
use super::EVENT_TABLE_ENTRY_ROWS;
use crate::circuits::cell::CellExpression;
use crate::circuits::utils::bn_to_field;
use crate::circuits::utils::step_status::Status;
use crate::circuits::utils::step_status::StepStatus;
use crate::circuits::utils::table_entry::EventTableEntryWithMemoryInfo;
use crate::circuits::utils::table_entry::EventTableWithMemoryInfo;
use crate::circuits::utils::Context;
use std::sync::Arc;

pub(in crate::circuits) struct EventTablePermutationCells {
    pub(in crate::circuits) rest_mops: Option<Cell>,
    pub(in crate::circuits) rest_jops: Option<Cell>,
    pub(in crate::circuits) fid_of_entry: Cell,
    pub(in crate::circuits) initial_memory_pages: Cell,
    pub(in crate::circuits) maximal_memory_pages: Cell,
}

impl<F: FieldExt> EventTableChip<F> {
    fn compute_rest_mops_and_jops(
        &self,
        op_configs: &BTreeMap<OpcodeClassPlain, Arc<EventTableOpcodeConfigBox<F>>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
    ) -> Vec<(u32, u32)> {
        let mut rest_ops = vec![];

        event_table
            .0
            .iter()
            .rev()
            .fold((0, 0), |(rest_mops_sum, rest_jops_sum), entry| {
                let instruction = entry.eentry.get_instruction(itable);

                let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();

                let acc = (
                    rest_mops_sum + op_config.0.memory_writing_ops(&entry.eentry),
                    rest_jops_sum + op_config.0.jops(),
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

    fn assign_entries(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Arc<EventTableOpcodeConfigBox<F>>>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        fid_of_entry: u32,
        rest_ops: Vec<(u32, u32)>,
    ) -> Result<(Cell, Cell, Cell), Error> {
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

        macro_rules! assign_constant {
            ($cell:ident, $value:expr) => {
                ctx.region.assign_advice_from_constant(
                    || "etable".to_owned() + stringify!($cell),
                    self.config.common_config.$cell.0.col,
                    ctx.offset + self.config.common_config.$cell.0.rot as usize,
                    $value,
                )?
            };
        }


        let mut host_public_inputs = 1u32;
        let mut context_in_index = 1u32;
        let mut context_out_index = 1u32;
        let mut external_host_call_call_index = 1u32;

        assign_constant!(input_index_cell, F::from(host_public_inputs as u64));
        assign_constant!(context_input_index_cell, F::from(context_in_index as u64));
        assign_constant!(context_output_index_cell, F::from(context_out_index as u64));
        assign_constant!(
            external_host_call_index_cell,
            F::from(external_host_call_call_index as u64)
        );
        let initial_memory_pages_cell = assign_advice!(
            mpages_cell,
            F::from(configure_table.init_memory_pages as u64)
        );
        let maximal_memory_pages_cell = assign_advice!(
            maximal_memory_pages_cell,
            F::from(configure_table.maximal_memory_pages as u64)
        );
        assign_constant!(sp_cell, F::from(DEFAULT_VALUE_STACK_LIMIT as u64 - 1));
        assign_constant!(frame_id_cell, F::zero());
        assign_constant!(eid_cell, F::one());
        let fid_of_entry_cell = assign_advice!(fid_cell, F::from(fid_of_entry as u64));
        assign_constant!(iid_cell, F::zero());

        /*
         * Skip subsequent advice assignment in the first pass to enhance performance.
         */
        {
            let assigned_cell = assign_advice!(enabled_cell, F::zero());
            if assigned_cell.value().is_none() {
                return Ok((
                    fid_of_entry_cell.cell(),
                    initial_memory_pages_cell.cell(),
                    maximal_memory_pages_cell.cell(),
                ));
            }
        }

        /*
         * The length of event_table equals 0: without_witness
         */
        if event_table.0.len() == 0 {
            return Ok((
                fid_of_entry_cell.cell(),
                initial_memory_pages_cell.cell(),
                maximal_memory_pages_cell.cell(),
            ));
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
                eid: status.last().unwrap().eid + 1,
                fid: 0,
                iid: 0,
                sp: status.last().unwrap().sp
                    + if let StepInfo::Return { drop, .. } =
                        &event_table.0.last().unwrap().eentry.step_info
                    {
                        *drop
                    } else {
                        unreachable!()
                    },
                last_jump_eid: 0,
                allocated_memory_pages: status.last().unwrap().allocated_memory_pages,
                itable,
            };

            status.push(terminate_status);

            status
        };

        use rayon::prelude::*;

        let etable: Vec<EventTableEntryWithMemoryInfo> = event_table.0.clone();


        let instructions =
            etable.iter().enumerate().map(|(index, entry)| {
                let step_status = StepStatus {
                    current: &status[index],
                    next: &status[index + 1],
                    current_external_host_call_index: external_host_call_call_index,
                    configure_table: *configure_table,
                    host_public_inputs,
                    context_in_index,
                    context_out_index,
                };
                let instruction = entry.eentry.get_instruction(itable);
                let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();
                let arg_context = (host_public_inputs, context_in_index, context_out_index, external_host_call_call_index);
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
                (entry, instruction, step_status, arg_context)
            }).zip(rest_ops.iter()).collect::<Vec<_>>();

        println!("instruction length {}", instructions.len());
        let chunk_len = instructions.len()/4;
        let chunk_len = if chunk_len == 0 { instructions.len() } else { chunk_len };

        let chunks = instructions.chunks(chunk_len).map(|x| (x, ctx.clone())).collect::<Vec<_>>();

        let timer = start_timer!(|| "par time");
        let _ = chunks.par_iter().enumerate().map(|(index, (instructions, ctx))| {
            println!("chunk size {}", instructions.len());
            let mut _ctx = ctx.clone();
            let ctx = &mut _ctx;
            ctx.step(EVENT_TABLE_ENTRY_ROWS as usize * index * chunk_len);
            let timer = start_timer!(|| format!("par time {}", index));
            let _ = instructions.iter().map(

                |((entry, instruction, step_status, arg_context), (rest_mops, rest_jops))| {

                    let (host_public_inputs, context_in_index, context_out_index, external_host_call_call_index) = *arg_context;

                    {
                        let class: OpcodeClassPlain = (&instruction.opcode).into();

                        let op = self.config.common_config.ops[class.index()];
                        op.assign(ctx, F::one())?;
                        //assign_advice_cell!(op, F::one());
                    }

                    self.config.common_config.enabled_cell.assign(ctx, F::one())?;
                    self.config.common_config.rest_mops_cell.assign(ctx, F::from(*rest_mops as u64))?;
                    self.config.common_config.rest_jops_cell.assign(ctx, F::from(*rest_jops as u64))?;
                    self.config.common_config.input_index_cell.assign(ctx, F::from(host_public_inputs as u64))?;
                    self.config.common_config.context_input_index_cell.assign(ctx, F::from(context_in_index as u64))?;
                    self.config.common_config.context_output_index_cell.assign(ctx, F::from(context_out_index as u64))?;
                    self.config.common_config.external_host_call_index_cell.assign(
                        ctx,
                        F::from(external_host_call_call_index as u64)
                        )?;
                    self.config.common_config.sp_cell.assign(ctx, F::from(entry.eentry.sp as u64))?;
                    self.config.common_config.mpages_cell.assign(
                        ctx,
                        F::from(entry.eentry.allocated_memory_pages as u64)
                        )?;
                    self.config.common_config.maximal_memory_pages_cell.assign(
                        ctx,
                        F::from(configure_table.maximal_memory_pages as u64)
                        )?;
                    self.config.common_config.frame_id_cell.assign(ctx, F::from(entry.eentry.last_jump_eid as u64))?;
                    self.config.common_config.eid_cell.assign(ctx, F::from(entry.eentry.eid as u64))?;
                    self.config.common_config.fid_cell.assign(ctx, F::from(entry.eentry.fid as u64))?;
                    self.config.common_config.iid_cell.assign(ctx, F::from(entry.eentry.iid as u64))?;
                    self.config.common_config.itable_lookup_cell.assign(ctx, bn_to_field(&instruction.encode))?;

                    let op_config = op_configs.get(&((&instruction.opcode).into())).unwrap();
                    op_config.0.assign(ctx, &step_status, &entry)?;


                    ctx.step(EVENT_TABLE_ENTRY_ROWS as usize);
                    Ok::<(), Error>(())
                }).collect::<Vec<_>>();
            end_timer!(timer);
        }).collect::<Vec<_>>();
        end_timer!(timer);

        ctx.step(EVENT_TABLE_ENTRY_ROWS as usize * instructions.len());

        // Assign terminate status
        assign_advice!(eid_cell, F::from(status.last().unwrap().eid as u64));
        assign_advice!(fid_cell, F::from(status.last().unwrap().fid as u64));
        assign_advice!(iid_cell, F::from(status.last().unwrap().iid as u64));
        assign_advice!(sp_cell, F::from(status.last().unwrap().sp as u64));
        assign_advice!(
            frame_id_cell,
            F::from(status.last().unwrap().last_jump_eid as u64)
        );
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

        Ok((
            fid_of_entry_cell.cell(),
            initial_memory_pages_cell.cell(),
            maximal_memory_pages_cell.cell(),
        ))
    }

    pub(in crate::circuits) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        itable: &InstructionTable,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        fid_of_entry: u32,
    ) -> Result<EventTablePermutationCells, Error> {
        debug!("size of execution table: {}", event_table.0.len());
        assert!(event_table.0.len() * EVENT_TABLE_ENTRY_ROWS as usize <= self.max_available_rows);

        let rest_ops =
            self.compute_rest_mops_and_jops(&self.config.op_configs, itable, event_table);

        self.init(ctx)?;
        ctx.reset();

        let (rest_mops_cell, rest_jops_cell) = self.assign_rest_ops_first_step(
            ctx,
            rest_ops.first().map_or(0u32, |(rest_mops, _)| *rest_mops),
            rest_ops.first().map_or(0u32, |(_, rest_jops)| *rest_jops),
        )?;
        ctx.reset();

        let (fid_of_entry, initial_memory_pages, maximal_memory_pages) = self.assign_entries(
            ctx,
            &self.config.op_configs,
            itable,
            event_table,
            configure_table,
            fid_of_entry,
            rest_ops,
        )?;
        ctx.reset();

        Ok(EventTablePermutationCells {
            rest_mops: Some(rest_mops_cell),
            rest_jops: Some(rest_jops_cell),
            fid_of_entry,
            initial_memory_pages,
            maximal_memory_pages,
        })
    }
}
