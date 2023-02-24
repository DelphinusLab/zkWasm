use halo2_proofs::{arithmetic::FieldExt, circuit::Cell, plonk::Error};
use specs::{configure_table::ConfigureTable, itable::OpcodeClassPlain};
use std::{collections::BTreeMap, rc::Rc};

use super::{EventTableChip, EventTableOpcodeConfig, EVENT_TABLE_ENTRY_ROWS};
use crate::circuits::{
    cell::CellExpression,
    utils::{
        bn_to_field,
        step_status::{Status, StepStatus},
        table_entry::{EventTableEntryWithMemoryInfo, EventTableWithMemoryInfo},
        Context,
    },
};

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
            .fold((0, 0), |(rest_mops_sum, rest_jops_sum), entry| {
                let op_config = op_configs
                    .get(&entry.eentry.inst.opcode.clone().into())
                    .unwrap();

                let acc = (
                    rest_mops_sum + op_config.memory_writing_ops(&entry.eentry),
                    rest_jops_sum + op_config.jops(),
                );

                rest_ops.push((acc.0, acc.1));

                acc
            });

        rest_ops.reverse();

        rest_ops
    }

    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for index in 0..self.max_available_rows {
            ctx.region.assign_fixed(
                || "etable: sel",
                self.config.sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if index % (EVENT_TABLE_ENTRY_ROWS as usize) == 0 {
                ctx.region.assign_fixed(
                    || "etable: step sel",
                    self.config.step_sel,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            ctx.next();
        }

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

    fn assign_advice(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
        rest_ops: Vec<(u32, u32)>,
    ) -> Result<(), Error> {
        macro_rules! assign_advice {
            ($cell:ident, $value:expr) => {
                self.config.common_config.$cell.assign(ctx, $value)?;
            };
        }

        macro_rules! assign_advice_cell {
            ($cell:ident, $value:expr) => {
                $cell.assign(ctx, $value)?;
            };
        }

        macro_rules! assign_constant {
            ($cell:ident, $value:expr) => {
                ctx.region.assign_advice_from_constant(
                    || "etable".to_owned() + stringify!($cell),
                    self.config.common_config.$cell.0.col,
                    ctx.offset + self.config.common_config.$cell.0.rot as usize,
                    $value,
                )?;
            };
        }

        let mut host_public_inputs = 0u32;
        let mut external_host_call_call_index = 1u32;
        let mut index = 0;

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

            status.push(Status {
                eid: 0,
                fid: 0,
                iid: 0,
                sp: 0,
                last_jump_eid: 0,
                allocated_memory_pages: 0,
            });

            status
        };

        assign_constant!(input_index_cell, F::from(host_public_inputs as u64));
        assign_constant!(
            external_host_call_index_cell,
            F::from(external_host_call_call_index as u64)
        );
        assign_constant!(
            mpages_cell,
            F::from(configure_table.init_memory_pages as u64)
        );

        for (entry, (rest_mops, rest_jops)) in event_table.0.iter().zip(rest_ops.iter()) {
            let step_status = StepStatus {
                current: &status[index],
                next: &status[index + 1],
                current_external_host_call_index: external_host_call_call_index,
                configure_table: *configure_table,
            };

            assign_advice!(enabled_cell, F::one());

            {
                let (op_lvl1, op_lvl2) = self
                    .config
                    .common_config
                    .allocate_opcode_bit_cell(entry.eentry.inst.opcode.clone().into());

                assign_advice_cell!(op_lvl1, F::one());
                assign_advice_cell!(op_lvl2, F::one());
            }

            assign_advice!(rest_mops_cell, F::from(*rest_mops as u64));
            assign_advice!(rest_jops_cell, F::from(*rest_jops as u64));
            assign_advice!(input_index_cell, F::from(host_public_inputs as u64));
            assign_advice!(
                external_host_call_index_cell,
                F::from(external_host_call_call_index as u64)
            );
            assign_advice!(sp_cell, F::from(entry.eentry.sp as u64));
            assign_advice!(
                mpages_cell,
                F::from(entry.eentry.allocated_memory_pages as u64)
            );
            assign_advice!(frame_id_cell, F::from(entry.eentry.last_jump_eid as u64));
            assign_advice!(eid_cell, F::from(entry.eentry.eid as u64));
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
            if op_config.is_external_host_call(&entry.eentry) {
                external_host_call_call_index += 1;
            }

            ctx.step(EVENT_TABLE_ENTRY_ROWS as usize);
            index += 1;
        }

        Ok(())
    }

    pub(crate) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        event_table: &EventTableWithMemoryInfo,
        configure_table: &ConfigureTable,
    ) -> Result<(Option<Cell>, Option<Cell>), Error> {
        let rest_ops = self.compute_rest_mops_and_jops(&self.config.op_configs, event_table);

        self.init(ctx)?;
        ctx.reset();

        let (rest_mops_cell, rest_jops_cell) = self.assign_rest_ops_first_step(
            ctx,
            rest_ops.first().unwrap().0,
            rest_ops.first().unwrap().1,
        )?;
        ctx.reset();

        self.assign_advice(
            ctx,
            &self.config.op_configs,
            event_table,
            configure_table,
            rest_ops,
        )?;
        ctx.reset();

        Ok((Some(rest_mops_cell), Some(rest_jops_cell)))
    }
}
