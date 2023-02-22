use halo2_proofs::{arithmetic::FieldExt, plonk::Error};
use specs::{etable::EventTable, itable::OpcodeClassPlain};
use std::{collections::BTreeMap, rc::Rc};

use super::{EventTableChip, EventTableOpcodeConfig, ESTEP_SIZE};
use crate::circuits::utils::{
    bn_to_field, table_entry::EventTableEntryWithMemoryReadingTable, Context,
};

impl<F: FieldExt> EventTableChip<F> {
    fn init(&self, ctx: &mut Context<'_, F>) -> Result<(), Error> {
        for index in 0..self.max_available_rows {
            ctx.region.assign_fixed(
                || "etable: sel",
                self.config.sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if index % (ESTEP_SIZE as usize) == 0 {
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

    fn assign_advice(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        event_table: &EventTableEntryWithMemoryReadingTable,
    ) -> Result<(), Error> {
        let mut host_public_inputs = 0u32;
        let mut external_host_call_call_index = 1u32;

        for entry in &event_table.0 {
            macro_rules! assign_advice {
                ($cell:ident, $value:expr) => {
                    ctx.region.assign_advice(
                        || "etable".to_owned() + stringify!($cell),
                        self.config.common_config.$cell.0.col,
                        ctx.offset + self.config.common_config.$cell.0.rot as usize,
                        || Ok($value),
                    )?;
                };
            }

            macro_rules! assign_advice_cell {
                ($cell:ident, $value:expr) => {
                    ctx.region.assign_advice(
                        || "etable".to_owned() + stringify!($cell),
                        $cell.0.col,
                        ctx.offset + $cell.0.rot as usize,
                        || Ok($value),
                    )?;
                };
            }

            assign_advice!(enabled_cell, F::one());

            {
                let (op_lvl1, op_lvl2) = self
                    .config
                    .common_config
                    .allocate_opcode_bit_cell(entry.eentry.inst.opcode.clone().into());

                assign_advice_cell!(op_lvl1, F::one());
                assign_advice_cell!(op_lvl2, F::one());
            }

            assign_advice!(rest_mops_cell, todo!());
            assign_advice!(rest_jops_cell, todo!());
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
            op_config.assign(ctx, todo!(), &entry.eentry)?;

            if op_config.is_host_public_input(&entry.eentry) {
                host_public_inputs += 1;
            }
            if op_config.is_external_host_call(&entry.eentry) {
                external_host_call_call_index += 1;
            }

            ctx.step(ESTEP_SIZE as usize);
        }

        Ok(())
    }

    pub(crate) fn assign(
        self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        event_table: &EventTableEntryWithMemoryReadingTable,
    ) -> Result<(), Error> {
        self.init(ctx)?;

        ctx.reset();

        self.assign_advice(ctx, op_configs, event_table)?;

        Ok(())
    }
}
