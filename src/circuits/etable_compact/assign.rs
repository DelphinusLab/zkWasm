use crate::circuits::rtable::RangeTableMixColumn;

use super::*;

impl<F: FieldExt> EventTableCommonConfig<F> {
    pub(super) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClassPlain, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        etable: &EventTable,
    ) -> Result<(Option<Cell>, Option<Cell>), Error> {
        let mut status_entries = Vec::with_capacity(etable.entries().len() + 1);

        // Step 1: fill fixed columns
        for _ in 0..MAX_ETABLE_ROWS {
            ctx.region.as_ref().borrow_mut().assign_fixed(
                || "etable common sel",
                self.sel,
                ctx.offset,
                || Ok(F::one()),
            )?;

            if ctx.offset % ETABLE_STEP_SIZE == EventTableBitColumnRotation::Enable as usize {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "etable common block first line sel",
                    self.block_first_line_sel,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            if ctx.offset % ETABLE_STEP_SIZE
                == EventTableUnlimitColumnRotation::ITableLookup as usize
            {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "itable lookup",
                    self.itable_lookup,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            if ctx.offset % ETABLE_STEP_SIZE
                == EventTableUnlimitColumnRotation::JTableLookup as usize
            {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "jtable lookup",
                    self.jtable_lookup,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            if ctx.offset % ETABLE_STEP_SIZE
                == EventTableUnlimitColumnRotation::PowTableLookup as usize
            {
                self.aux.assign_lookup(
                    &mut ctx.region.as_ref().borrow_mut(),
                    ctx.offset,
                    RangeTableMixColumn::Pow,
                )?;
            }

            if ctx.offset % ETABLE_STEP_SIZE
                == EventTableUnlimitColumnRotation::OffsetLenBitsTableLookup as usize
            {
                self.aux.assign_lookup(
                    &mut ctx.region.as_ref().borrow_mut(),
                    ctx.offset,
                    RangeTableMixColumn::OffsetLenBits,
                )?;
            }

            if ctx.offset % ETABLE_STEP_SIZE
                >= EventTableUnlimitColumnRotation::MTableLookupStart as usize
                && ctx.offset % ETABLE_STEP_SIZE
                    < EventTableUnlimitColumnRotation::U64Start as usize
            {
                ctx.region.as_ref().borrow_mut().assign_fixed(
                    || "mtable lookup",
                    self.mtable_lookup,
                    ctx.offset,
                    || Ok(F::one()),
                )?;
            }

            ctx.next();
        }

        ctx.reset();

        let mut rest_mops_cell: Option<Cell> = None;
        let mut rest_jops_cell: Option<Cell> = None;

        let mut mops = vec![];
        let mut jops = vec![];
        let mut host_public_inputs = 0u64;

        macro_rules! assign_advice {
            ($c:expr, $o:expr, $k:expr, $v:expr) => {
                ctx.region.as_ref().borrow_mut().assign_advice(
                    || $k,
                    $c,
                    ctx.offset + $o as usize,
                    || Ok(F::from($v)),
                )?
            };
        }

        macro_rules! assign_constant {
            ($c:expr, $o:expr, $k:expr, $v:expr) => {
                ctx.region
                    .as_ref()
                    .borrow_mut()
                    .assign_advice_from_constant(|| $k, $c, ctx.offset + $o as usize, F::from($v))?
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

            if index == 0 {
                assign_constant!(
                    self.state,
                    EventTableCommonRangeColumnRotation::InputIndex,
                    "input index",
                    F::zero()
                );
            } else {
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

            let cell = assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::RestMOps,
                "rest mops",
                rest_mops.next().unwrap()
            );
            if rest_mops_cell.is_none() {
                rest_mops_cell = Some(cell.cell());
            }

            let cell = assign_advice!(
                self.state,
                EventTableCommonRangeColumnRotation::RestJOps,
                "rest jops",
                rest_jops.next().unwrap()
            );
            if rest_jops_cell.is_none() {
                rest_jops_cell = Some(cell.cell());
            }

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

            ctx.region.as_ref().borrow_mut().assign_advice(
                || "itable lookup entry",
                self.aux.internal,
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

        Ok((rest_mops_cell, rest_jops_cell))
    }
}
