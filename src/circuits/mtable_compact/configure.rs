use super::*;
use crate::constant_from;
use crate::curr;
use crate::fixed_curr;
use crate::nextn;
use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use specs::mtable::AccessType;
use specs::mtable::LocationType;

pub const STEP_SIZE: i32 = 9;

pub trait MemoryTableConstriants<F: FieldExt> {
    fn configure(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
    ) {
        self.configure_enable_as_bit(meta, rtable);
        self.configure_rest_mops_decrease(meta, rtable);
        self.configure_final_rest_mops_zero(meta, rtable);
        self.configure_read_nochange(meta, rtable);
        self.configure_heap_first_init(meta, rtable);
        self.configure_stack_first_not_read(meta, rtable);
        self.configure_index_sort(meta, rtable);
        /*
        self.configure_tvalue_bytes(meta, rtable);
        self.configure_heap_init_in_imtable(meta, rtable, imtable);
         */
    }

    fn configure_enable_as_bit(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>);
    fn configure_enable_seq(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>);
    fn configure_index_sort(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>);
    fn configure_rest_mops_decrease(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    );
    fn configure_final_rest_mops_zero(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    );
    fn configure_read_nochange(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>);
    fn configure_heap_first_init(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    );
    fn configure_stack_first_not_read(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
    );
    fn configure_tvalue_bytes(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>);
    fn configure_heap_init_in_imtable(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
    );
}

impl<F: FieldExt> MemoryTableConstriants<F> for MemoryTableConfig<F> {
    fn configure_enable_as_bit(
        &self,
        meta: &mut ConstraintSystem<F>,
        _rtable: &RangeTableConfig<F>,
    ) {
        meta.create_gate("mtable configure_enable_as_bit", |meta| {
            vec![
                curr!(meta, self.enable)
                    * (curr!(meta, self.enable) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
                curr!(meta, self.enable)
                    * (fixed_curr!(meta, self.block_first_line_sel) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn configure_enable_seq(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        meta.create_gate("mtable configure_enable_seq", |meta| {
            vec![
                nextn!(meta, self.enable, STEP_SIZE)
                    * (curr!(meta, self.enable) - constant_from!(1))
                    * fixed_curr!(meta, self.sel),
            ]
        });
    }

    fn configure_index_sort(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        meta.create_gate("mtable configure_index_same", |meta| {
            vec![
                curr!(meta, self.aux) - constant_from!(1),
                self.same_mmid(meta) - self.same_ltype(meta) * self.same_mmid_single(meta),
                self.same_offset(meta) - self.same_mmid(meta) * self.same_offset_single(meta),
                self.same_eid(meta) - self.same_offset(meta) * self.same_eid_single(meta),
            ]
            .into_iter()
            .map(|e| e * self.is_enabled_following_block(meta))
            .collect::<Vec<_>>()
        });

        rtable.configure_in_common_range(meta, "mtable configure_index_sort", |meta| {
            (curr!(meta, self.index.data) - nextn!(meta, self.index.data, -STEP_SIZE))
                * curr!(meta, self.aux)
                * self.is_enabled_following_block(meta)
        });

        rtable.configure_in_common_range(meta, "mtable configure_index_sort", |meta| {
            curr!(meta, self.index.data) * self.is_enabled_line(meta)
        });
    }

    fn configure_rest_mops_decrease(
        &self,
        meta: &mut ConstraintSystem<F>,
        _rtable: &RangeTableConfig<F>,
    ) {
        meta.create_gate(
            "mtable configure_rest_mops_decrease decrease on non-init",
            |meta| {
                vec![
                    (self.prev_rest_mops(meta) - self.rest_mops(meta) - constant_from!(1))
                        * (self.prev_atype(meta) - constant_from!(AccessType::Init)),
                ]
                .into_iter()
                .map(|e| e * self.is_enabled_following_block(meta))
                .collect::<Vec<_>>()
            },
        );

        meta.create_gate(
            "mtable configure_rest_mops_decrease no decrease on init",
            |meta| {
                vec![
                    (self.prev_rest_mops(meta) - self.rest_mops(meta))
                        * (self.prev_atype(meta) - constant_from!(AccessType::Write))
                        * (self.prev_atype(meta) - constant_from!(AccessType::Read)),
                ]
                .into_iter()
                .map(|e| e * self.is_enabled_following_block(meta))
                .collect::<Vec<_>>()
            },
        );
    }

    fn configure_final_rest_mops_zero(
        &self,
        meta: &mut ConstraintSystem<F>,
        _rtable: &RangeTableConfig<F>,
    ) {
        meta.create_gate("mtable configure_final_rest_mops_zero", |meta| {
            vec![self.rest_mops(meta) * (curr!(meta, self.enable) - constant_from!(1))]
                .into_iter()
                .map(|e| e * self.is_enabled_following_block(meta))
                .collect::<Vec<_>>()
        });
    }

    fn configure_read_nochange(
        &self,
        meta: &mut ConstraintSystem<F>,
        _rtable: &RangeTableConfig<F>,
    ) {
        meta.create_gate("mtable configure_read_nochange", |meta| {
            vec![
                (self.atype(meta) - constant_from!(AccessType::Write))
                    * (self.atype(meta) - constant_from!(AccessType::Init))
                    * self.same_offset(meta)
                    * (self.prev_value(meta) - self.value(meta)),
            ]
            .into_iter()
            .map(|e| e * self.is_enabled_following_block(meta))
            .collect::<Vec<_>>()
        });

        meta.create_gate("mtable configure_read_nochange", |meta| {
            vec![
                (self.atype(meta) - constant_from!(AccessType::Write))
                    * (self.atype(meta) - constant_from!(AccessType::Init))
                    * self.same_offset(meta)
                    * (self.prev_vtype(meta) - self.vtype(meta)),
            ]
            .into_iter()
            .map(|e| e * self.is_enabled_following_block(meta))
            .collect::<Vec<_>>()
        });
    }

    fn configure_heap_first_init(
        &self,
        meta: &mut ConstraintSystem<F>,
        _rtable: &RangeTableConfig<F>,
    ) {
        meta.create_gate("mtable configure_heap_first_init", |meta| {
            vec![
                (self.ltype(meta) - constant_from!(LocationType::Stack))
                    * (constant_from!(1) - self.same_offset(meta))
                    * (self.atype(meta) - constant_from!(AccessType::Init)),
            ]
            .into_iter()
            .map(|e| e * self.is_enabled_following_block(meta))
            .collect::<Vec<_>>()
        });
    }

    fn configure_stack_first_not_read(
        &self,
        meta: &mut ConstraintSystem<F>,
        _rtable: &RangeTableConfig<F>,
    ) {
        meta.create_gate("mtable configure_stack_first_write", |meta| {
            vec![
                (self.ltype(meta) - constant_from!(LocationType::Heap))
                    * (constant_from!(1) - self.same_offset(meta))
                    * (self.atype(meta) - constant_from!(AccessType::Write))
                    * (self.atype(meta) - constant_from!(AccessType::Init)),
            ]
            .into_iter()
            .map(|e| e * self.is_enabled_following_block(meta))
            .collect::<Vec<_>>()
        });
    }

    fn configure_tvalue_bytes(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        rtable.configure_in_u8_range(meta, "mtable bytes", |meta| {
            curr!(meta, self.bytes) * self.is_enabled_line(meta)
        });
        todo!()
    }

    fn configure_heap_init_in_imtable(
        &self,
        meta: &mut ConstraintSystem<F>,
        rtable: &RangeTableConfig<F>,
        imtable: &InitMemoryTableConfig<F>,
    ) {
        todo!()
    }
}

impl<F: FieldExt> MemoryTableConfig<F> {
    pub(super) fn new(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let sel = meta.fixed_column();
        let following_block_sel = meta.fixed_column();
        let block_first_line_sel = meta.fixed_column();
        let enable = cols.next().unwrap();
        let index = RowDiffConfig::configure("mtable index", meta, cols, STEP_SIZE, |meta| {
            fixed_curr!(meta, following_block_sel) * curr!(meta, enable)
        });
        let aux = cols.next().unwrap();
        let bytes = cols.next().unwrap();

        MemoryTableConfig {
            sel,
            following_block_sel,
            block_first_line_sel,
            enable,
            index,
            aux,
            bytes,
        }
    }
}
