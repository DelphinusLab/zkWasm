use std::{collections::BTreeMap, rc::Rc};

use halo2_proofs::{arithmetic::FieldExt, plonk::Error};
use specs::{etable::EventTable, itable::OpcodeClassPlain};

use crate::circuits::utils::{bn_to_field, Context};

use super::{EventTableChip, EventTableOpcodeConfig, ESTEP_SIZE};

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
        event_table: &EventTable,
    ) -> Result<(), Error> {
        let mut host_public_inputs = 0u32;
        let mut external_host_call_call_index = 1u32;

        for entry in event_table.entries() {
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

            assign_advice!(enabled_cell, 1);

            {
                let (op_lvl1, op_lvl2) = self
                    .config
                    .common_config
                    .allocate_opcode_bit_cell(entry.inst.opcode.clone().into());

                assign_advice_cell!(op_lvl1, 1);
                assign_advice_cell!(op_lvl2, 1);
            }

            assign_advice!(rest_mops_cell, todo!());
            assign_advice!(rest_jops_cell, todo!());
            assign_advice!(input_index_cell, F::from(host_public_inputs as u64));
            assign_advice!(
                external_host_call_index_cell,
                F::from(external_host_call_call_index as u64)
            );
            assign_advice!(sp_cell, F::from(entry.sp as u64));
            assign_advice!(mpages_cell, F::from(entry.allocated_memory_pages as u64));
            assign_advice!(frame_id_cell, F::from(entry.last_jump_eid as u64));
            assign_advice!(eid_cell, F::from(entry.eid as u64));
            assign_advice!(fid_cell, F::from(entry.inst.fid as u64));
            assign_advice!(iid_cell, F::from(entry.inst.iid as u64));
            assign_advice!(itable_lookup_cell, bn_to_field(&entry.inst.encode()));

            let op_config = op_configs.get(&entry.inst.opcode.clone().into()).unwrap();
            op_config.assign(ctx, todo!(), entry)?;

            if op_config.is_host_public_input(entry) {
                host_public_inputs += 1;
            }
            if op_config.is_external_host_call(entry) {
                external_host_call_call_index += 1;
            }

            ctx.step(ESTEP_SIZE);
        }

        Ok(())
    }

    pub fn assign(
        self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        event_table: &EventTable,
    ) -> Result<(), Error> {
        self.init(ctx)?;

        ctx.reset();

        self.assign_advice(ctx, op_configs, event_table)?;

        Ok(())
    }
}
