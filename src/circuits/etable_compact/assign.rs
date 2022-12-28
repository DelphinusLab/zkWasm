use crate::circuits::config::max_etable_rows;

use super::*;

impl<F: FieldExt> EventTableCommonConfig<F> {
    pub(super) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        etable: &EventTable,
    ) -> Result<(Option<Cell>, Option<Cell>), Error> {
        let mut status_entries = Vec::with_capacity(etable.entries().len() + 1);

        // Ensure capability, at least reserve one row for disabled entries
        assert!(etable.entries().len() * ETABLE_STEP_SIZE < max_etable_rows() as usize);

        // Step 1: fill fixed columns
        for i in 0..max_etable_rows() as usize {
            ctx.region
                .assign_fixed(|| "etable common sel", self.sel, i, || Ok(F::one()))?;

            if i % ETABLE_STEP_SIZE == EventTableBitColumnRotation::Enable as usize {
                ctx.region.assign_fixed(
                    || "etable common block first line sel",
                    self.block_first_line_sel,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE == EventTableUnlimitColumnRotation::ITableLookup as usize {
                ctx.region.assign_fixed(
                    || "itable lookup",
                    self.itable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE == EventTableUnlimitColumnRotation::BrTableLookup as usize {
                ctx.region.assign_fixed(
                    || "brtable lookup",
                    self.brtable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE == EventTableUnlimitColumnRotation::JTableLookup as usize {
                ctx.region.assign_fixed(
                    || "jtable lookup",
                    self.jtable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE == EventTableUnlimitColumnRotation::PowTableLookup as usize {
                ctx.region.assign_fixed(
                    || "pow table lookup",
                    self.pow_table_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE
                == EventTableUnlimitColumnRotation::OffsetLenBitsTableLookup as usize
            {
                ctx.region.assign_fixed(
                    || "offset len bits table lookup",
                    self.offset_len_bits_table_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }

            if i % ETABLE_STEP_SIZE >= EventTableUnlimitColumnRotation::MTableLookupStart as usize
                && i % ETABLE_STEP_SIZE < EventTableUnlimitColumnRotation::U64Start as usize
            {
                ctx.region.assign_fixed(
                    || "mtable lookup",
                    self.mtable_lookup,
                    i,
                    || Ok(F::one()),
                )?;
            }
        }

        let rest_mops_cell = ctx.region.assign_advice(
            || "rest mops",
            self.state,
            EventTableCommonRangeColumnRotation::RestMOps as usize,
            || Ok(F::from(0u64)),
        )?;
        let rest_jops_cell = ctx.region.assign_advice(
            || "rest mops",
            self.state,
            EventTableCommonRangeColumnRotation::RestJOps as usize,
            || Ok(F::from(0u64)),
        )?;

        let mut mops = vec![];
        let mut jops = vec![];
        let mut host_public_inputs = 0u64;

        macro_rules! assign_advice {
            ($c:expr, $o:expr, $k:expr, $v:expr) => {
                ctx.region
                    .assign_advice(|| $k, $c, ctx.offset + $o as usize, || Ok(F::from($v)))?
            };
        }

        macro_rules! assign_constant {
            ($c:expr, $o:expr, $k:expr, $v:expr) => {
                ctx.region.assign_advice_from_constant(
                    || $k,
                    $c,
                    ctx.offset + $o as usize,
                    F::from($v),
                )?
            };
        }

        for entry in etable.entries().iter() {
            status_entries.push(Status {
                eid: entry.eid,
                moid: entry.inst.moid,
                fid: entry.inst.fid,
                iid: entry.inst.iid,
                mmid: entry.inst.mmid,
                sp: entry.sp,
                last_jump_eid: entry.last_jump_eid,
            });
        }

        status_entries.push(Status {
            eid: 0,
            moid: 0,
            fid: 0,
            iid: 0,
            mmid: 0,
            sp: 0,
            last_jump_eid: 0,
        });

        let mut mops_in_total = 0;
        let mut jops_in_total = 0;

        assign_constant!(
            self.state,
            EventTableCommonRangeColumnRotation::InputIndex,
            "input index",
            F::zero()
        );

        for (index, entry) in etable.entries().iter().enumerate() {
            let opcode: OpcodeClassPlain = entry.inst.opcode.clone().into();

            let step_status = StepStatus {
                current: &status_entries[index],
                next: &status_entries[index + 1],
            };

            let config = op_configs.get(&opcode).unwrap();

            config.assign(ctx, &step_status, entry)?;

            mops.push(
                config.assigned_extra_mops(ctx, &step_status, entry) + entry.inst.opcode.mops(),
            );
            jops.push(entry.inst.opcode.jops());

            mops_in_total += mops.last().unwrap();
            jops_in_total += jops.last().unwrap();

            if index != 0 {
                assign_advice!(
                    self.state,
                    EventTableCommonRangeColumnRotation::InputIndex,
                    "input index",
                    host_public_inputs
                );
            }

            if config.is_host_public_input(&step_status, entry) {
                host_public_inputs += 1;
            }

            for _ in 0..ETABLE_STEP_SIZE {
                ctx.next();
            }
        }

        ctx.reset();

        mops.iter_mut().for_each(|x| {
            let t = *x;
            *x = mops_in_total;
            mops_in_total -= t;
        });

        // Ignore the last return
        jops_in_total -= 1;
        jops.iter_mut().for_each(|x| {
            let t = *x;
            *x = jops_in_total;
            jops_in_total = jops_in_total.checked_sub(t).unwrap_or(0);
        });

        let mut rest_mops = mops.into_iter();
        let mut rest_jops = jops.into_iter();

        // Step: fill Status for each eentry

        for entry in etable.entries().iter() {
            let opcode: OpcodeClassPlain = entry.inst.opcode.clone().into();

            assign_advice!(
                self.shared_bits[0],
                EventTableBitColumnRotation::Enable,
                "shared_bits",
                1
            );

            {
                let (op_lvl1, op_lvl2) = opclass_to_two_level(opcode);

                assign_advice!(self.opcode_bits, op_lvl1, "opcode level 1", 1);
                assign_advice!(self.opcode_bits, op_lvl2, "opcode level 2", 1);
            }

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::RestMOps,
                "rest mops",
                rest_mops.next().unwrap()
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::RestJOps,
                "rest jops",
                rest_jops.next().unwrap()
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::EID,
                "eid",
                entry.eid
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::MOID,
                "moid",
                entry.inst.moid as u64
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::FID,
                "fid",
                entry.inst.fid as u64
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::IID,
                "iid",
                entry.inst.iid as u64
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::MMID,
                "mmid",
                entry.inst.mmid as u64
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::SP,
                "sp",
                entry.sp
            );

            assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::LastJumpEid,
                "last jump eid",
                entry.last_jump_eid
            );

            ctx.region.assign_advice(
                || "itable lookup entry",
                self.aux,
                ctx.offset + EventTableUnlimitColumnRotation::ITableLookup as usize,
                || Ok(bn_to_field(&entry.inst.encode())),
            )?;

            for _ in 0..ETABLE_STEP_SIZE {
                ctx.next();
            }
        }

        // Step 3: fill the first disabled row

        {
            assign_advice!(
                self.shared_bits[0],
                EventTableBitColumnRotation::Enable,
                "shared_bits",
                0
            );
        }

        Ok((Some(rest_mops_cell.cell()), Some(rest_jops_cell.cell())))
    }
}
