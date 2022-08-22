use super::*;

impl<F: FieldExt> EventTableCommonConfig<F> {
    pub(super) fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        op_configs: &BTreeMap<OpcodeClass, Rc<Box<dyn EventTableOpcodeConfig<F>>>>,
        etable: &EventTable,
    ) -> Result<(Option<Cell>, Option<Cell>), Error> {
        let mut status_entries = Vec::with_capacity(etable.entries().len() + 1);

        // Step 1: fill fixed columns
        for i in 0..MAX_ETABLE_ROWS {
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
                    || "pow table lookup",
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

        let mut rest_mops_cell: Option<Cell> = None;
        let mut rest_jops_cell: Option<Cell> = None;
        let mut rest_mops = etable.rest_mops();
        let mut rest_jops = etable.rest_jops();

        macro_rules! assign_advice {
            ($c:expr, $o:expr, $k:expr, $v:expr) => {
                ctx.region
                    .assign_advice(|| $k, $c, ctx.offset + $o as usize, || Ok(F::from($v)))?
            };
        }

        // Step 2: fill Status for each eentry

        for entry in etable.entries().iter() {
            let opcode: OpcodeClass = entry.inst.opcode.clone().into();

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

            ctx.region.assign_advice(
                || "itable lookup entry",
                self.aux,
                ctx.offset + EventTableUnlimitColumnRotation::ITableLookup as usize,
                || Ok(bn_to_field(&entry.inst.encode())),
            )?;

            status_entries.push(Status {
                eid: entry.eid,
                moid: entry.inst.moid,
                fid: entry.inst.fid,
                iid: entry.inst.iid,
                mmid: entry.inst.mmid,
                sp: entry.sp,
                last_jump_eid: entry.last_jump_eid,
            });

            for _ in 0..ETABLE_STEP_SIZE {
                ctx.next();
            }
        }

        // Step 3: fill the first disabled row

        {
            status_entries.push(Status {
                eid: 0,
                moid: 0,
                fid: 0,
                iid: 0,
                mmid: 0,
                sp: 0,
                last_jump_eid: 0,
            });

            assign_advice!(
                self.shared_bits[0],
                EventTableBitColumnRotation::Enable,
                "shared_bits",
                0
            );
        }

        // Step 4: fill lookup aux

        ctx.reset();

        for (index, entry) in etable.entries().iter().enumerate() {
            let opcode: OpcodeClass = entry.inst.opcode.clone().into();

            let step_status = StepStatus {
                current: &status_entries[index],
                next: &status_entries[index + 1],
            };

            let config = op_configs.get(&opcode).unwrap();

            config.assign(ctx, &step_status, entry)?;

            for _ in 0..ETABLE_STEP_SIZE {
                ctx.next();
            }
        }

        Ok((rest_mops_cell, rest_jops_cell))
    }
}
