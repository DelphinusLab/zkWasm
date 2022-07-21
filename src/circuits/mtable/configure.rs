use super::*;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::next;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use specs::mtable::AccessType;

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(super) fn new(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
    ) -> Self {
        let sel = meta.fixed_column();
        let enable = cols.next().unwrap();
        let emid = RowDiffConfig::configure("mtable emid", meta, cols, |meta| {
            next!(meta, enable) * fixed_curr!(meta, sel)
        });
        let ltype = RowDiffConfig::configure("mtable ltype", meta, cols, |meta| {
            next!(meta, enable) * fixed_curr!(meta, sel)
        });
        let mmid = RowDiffConfig::configure("mtable mmid", meta, cols, |meta| {
            next!(meta, enable) * fixed_curr!(meta, sel)
        });
        let offset = RowDiffConfig::configure("mtable offset", meta, cols, |meta| {
            next!(meta, enable) * fixed_curr!(meta, sel)
        });
        let eid = RowDiffConfig::configure("mtable eid", meta, cols, |meta| {
            next!(meta, enable) * fixed_curr!(meta, sel)
        });
        let tvalue = TValueConfig::configure(meta, cols, rtable, |meta| {
            next!(meta, enable) * fixed_curr!(meta, sel)
        });
        let atype = cols.next().unwrap();
        let same_location = cols.next().unwrap();
        let rest_mops = cols.next().unwrap();

        meta.enable_equality(rest_mops);

        MemoryTableConfig {
            sel,
            ltype,
            mmid,
            offset,
            eid,
            emid,
            atype,
            tvalue,
            enable,
            same_location,
            rest_mops,
        }
    }

    pub(super) fn configure_enable(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("enable seq", |meta| {
            vec![
                next!(meta, self.enable)
                    * (curr!(meta, self.enable) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    pub(super) fn configure_same_location(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("is same location", |meta| {
            let same_loc = curr!(meta, self.same_location);
            vec![
                self.ltype.is_same(meta) * self.mmid.is_same(meta) * self.offset.is_same(meta)
                    - same_loc,
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, self.sel))
            .collect::<Vec<_>>()
        })
    }

    pub(super) fn configure_range(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    ) {
        // value is range is limited by U64Config

        rtable.configure_in_common_range(meta, "mmid in range", |meta| self.mmid.data(meta));
        rtable.configure_in_common_range(meta, "offset in range", |meta| self.offset.data(meta));
        rtable.configure_in_common_range(meta, "eid in range", |meta| self.eid.data(meta));
        rtable.configure_in_common_range(meta, "emid in range", |meta| self.emid.data(meta));
        rtable.configure_in_u8_range(meta, "vtype in range", |meta| self.emid.data(meta));

        meta.create_gate("stack_or_heap", |meta| {
            vec![
                (self.ltype.data(meta) - constant_from!(LocationType::Stack as u64))
                    * (self.ltype.data(meta) - constant_from!(LocationType::Heap as u64))
                    * fixed_curr!(meta, self.sel),
            ]
        });

        meta.create_gate("enable is bit", |meta| {
            vec![
                curr!(meta, self.enable)
                    * (curr!(meta, self.enable) - constant_from!(1u64))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    pub(super) fn configure_sort(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    ) {
        rtable.configure_in_common_range(meta, "ltype sort", |meta| {
            self.is_next_enable(meta) * self.ltype.diff_to_next(meta) * fixed_curr!(meta, self.sel)
        });

        rtable.configure_in_common_range(meta, "mmid sort", |meta| {
            self.is_next_enable(meta)
                * self.ltype.is_next_same(meta)
                * self.mmid.diff_to_next(meta)
                * fixed_curr!(meta, self.sel)
        });
        rtable.configure_in_common_range(meta, "offset sort", |meta| {
            self.is_next_enable(meta)
                * self.ltype.is_next_same(meta)
                * self.mmid.is_next_same(meta)
                * self.offset.diff_to_next(meta)
                * fixed_curr!(meta, self.sel)
        });
        rtable.configure_in_common_range(meta, "eid sort", |meta| {
            self.is_next_enable(meta)
                * self.is_next_same_location(meta)
                * self.eid.diff_to_next(meta)
                * fixed_curr!(meta, self.sel)
        });
        rtable.configure_in_common_range(meta, "emid sort", |meta| {
            self.is_next_enable(meta)
                * self.is_same_location(meta)
                * self.eid.is_next_same(meta)
                * self.emid.diff_to_next(meta)
                * fixed_curr!(meta, self.sel)
        });
    }

    pub(super) fn configure_rule(
        &self,
        meta: &mut ConstraintSystem<F>,
        imtable: &InitMemoryTableConfig<F>,
    ) {
        meta.create_gate("mtable read after write", |meta| {
            vec![
                self.is_next_enable(meta)
                    * self.is_next_read_not_bit(meta)
                    * self.diff_to_next(meta, self.tvalue.value.value),
                self.is_next_enable(meta)
                    * self.is_next_read_not_bit(meta)
                    * self.diff_to_next(meta, self.tvalue.vtype),
            ]
            .into_iter()
            .map(|x| x * fixed_curr!(meta, self.sel))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mtable stack first line must be write", |meta| {
            vec![
                self.is_enable(meta)
                    * self.is_diff_location(meta)
                    * self.is_stack(meta)
                    * (curr!(meta, self.atype) - constant_from!(AccessType::Write))
                    * fixed_curr!(meta, self.sel),
            ]
        });

        imtable.configure_in_table(meta, "mtable heap first line", |meta| {
            self.is_enable(meta)
                * self.is_diff_location(meta)
                * self.is_heap(meta)
                * imtable.encode(
                    self.mmid.data(meta),
                    self.offset.data(meta),
                    curr!(meta, self.tvalue.value.value),
                )
                * fixed_curr!(meta, self.sel)
        });

        meta.create_gate("rest mop decrease", |meta| {
            vec![
                self.is_enable(meta)
                    * self.is_not_init(meta)
                    * (curr!(meta, self.rest_mops)
                        - next!(meta, self.rest_mops)
                        - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });

        meta.create_gate("rest mop zero when disabled", |meta| {
            vec![
                (self.is_enable(meta) - constant_from!(1))
                    * curr!(meta, self.rest_mops)
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }
}
