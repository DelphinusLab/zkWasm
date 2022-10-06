use crate::circuits::{config::POW_TABLE_LIMIT, rtable::offset_len_bits_encode};

use super::*;
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};

pub(super) mod op_bin;
pub(super) mod op_bin_bit;
pub(super) mod op_bin_shift;
pub(super) mod op_br;
pub(super) mod op_br_if;
pub(super) mod op_br_if_eqz;
pub(super) mod op_call;
pub(super) mod op_call_host_input;
pub(super) mod op_const;
pub(super) mod op_conversion;
pub(super) mod op_drop;
pub(super) mod op_load;
pub(super) mod op_local_get;
pub(super) mod op_local_set;
pub(super) mod op_local_tee;
pub(super) mod op_rel;
pub(super) mod op_return;
pub(crate) mod op_select;
pub(super) mod op_store;
pub(super) mod op_test;

// TODO: replace repeated code with macro

#[derive(Copy, Clone)]
pub struct UnlimitedCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl UnlimitedCell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, value: F) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(value),
        )?;
        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

pub struct MTableLookupCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl MTableLookupCell {
    pub fn assign<F: FieldExt>(
        &self,
        ctx: &mut Context<'_, F>,
        value: &BigUint,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "mlookup cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(bn_to_field(value)),
        )?;
        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

#[derive(Copy, Clone)]
pub struct OffsetLenBitsTableLookupCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl OffsetLenBitsTableLookupCell {
    pub fn assign<F: FieldExt>(
        &self,
        ctx: &mut Context<'_, F>,
        offset: u64,
        len: u64,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "offset len bits lookup cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(F::from(offset_len_bits_encode(offset, len))),
        )?;
        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

#[derive(Copy, Clone)]
pub struct PowTableLookupCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl PowTableLookupCell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, power: u64) -> Result<(), Error> {
        assert!(power < POW_TABLE_LIMIT);
        ctx.region.assign_advice(
            || "pow lookup cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || {
                Ok(bn_to_field(
                    &((BigUint::from(1u64) << (power + 16)) + power),
                ))
            },
        )?;
        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

pub struct JTableLookupCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl JTableLookupCell {
    pub fn assign<F: FieldExt>(
        &self,
        ctx: &mut Context<'_, F>,
        value: &BigUint,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "jlookup cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(bn_to_field(value)),
        )?;
        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

#[derive(Copy, Clone)]
pub struct BitCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl BitCell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, value: bool) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "bit cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(F::from(value as u64)),
        )?;

        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

#[derive(Clone, Copy)]
pub struct CommonRangeCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl CommonRangeCell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, value: u16) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "common range cell",
            self.col,
            (ctx.offset as i32 + self.rot) as usize,
            || Ok(F::from(value as u64)),
        )?;
        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, self.rot)
    }
}

#[derive(Clone, Copy)]
pub struct U4BopCell {
    pub col: Column<Advice>,
}

impl U4BopCell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, value: F) -> Result<(), Error> {
        for i in 0..16usize {
            ctx.region.assign_advice(
                || "u4 bop cell",
                self.col,
                ctx.offset + i,
                || Ok(F::from(value)),
            )?;
        }

        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.col, 0)
    }

    pub fn eq_constraint<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        let mut sum = constant_from!(0);
        for i in 1..16 {
            sum = sum + nextn!(meta, self.col, i);
        }
        sum - constant_from!(15) * nextn!(meta, self.col, 0)
    }
}

#[derive(Clone, Copy)]
pub struct U64BitCell {
    pub value_col: Column<Advice>,
    pub value_rot: i32,
    pub u4_col: Column<Advice>,
}

#[derive(Clone, Copy)]
pub struct U64Cell {
    pub value_col: Column<Advice>,
    pub value_rot: i32,
    pub u4_col: Column<Advice>,
}

impl U64Cell {
    pub fn assign<F: FieldExt>(
        &self,
        ctx: &mut Context<'_, F>,
        mut value: u64,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "u64 range cell",
            self.value_col,
            (ctx.offset as i32 + self.value_rot) as usize,
            || Ok(F::from(value)),
        )?;

        for i in 0..16usize {
            let v = value & 0xf;
            value >>= 4;
            ctx.region.assign_advice(
                || "u4 range cell",
                self.u4_col,
                ctx.offset + i,
                || Ok(F::from(v)),
            )?;
        }

        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.value_col, self.value_rot)
    }

    pub fn u4_expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>, i: i32) -> Expression<F> {
        nextn!(meta, self.u4_col, i)
    }
}

#[derive(Clone, Copy)]
pub struct U64OnU8Cell {
    pub value_col: Column<Advice>,
    pub value_rot: i32,
    pub u8_col: Column<Advice>,
    pub u8_rot: i32,
}

impl U64OnU8Cell {
    pub fn assign<F: FieldExt>(
        &self,
        ctx: &mut Context<'_, F>,
        mut value: u64,
    ) -> Result<(), Error> {
        ctx.region.assign_advice(
            || "u64 range cell",
            self.value_col,
            (ctx.offset as i32 + self.value_rot) as usize,
            || Ok(F::from(value)),
        )?;

        for i in 0..8usize {
            let v = value & 0xff;
            value >>= 8;
            ctx.region.assign_advice(
                || "u8 range cell",
                self.u8_col,
                ((ctx.offset + i) as i32 + self.u8_rot) as usize,
                || Ok(F::from(v)),
            )?;
        }

        Ok(())
    }

    pub fn expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        nextn!(meta, self.value_col, self.value_rot)
    }

    pub fn u8_expr<F: FieldExt>(&self, meta: &mut VirtualCells<'_, F>, i: i32) -> Expression<F> {
        nextn!(meta, self.u8_col, i + self.u8_rot)
    }
}

pub struct EventTableCellAllocator<'a, F> {
    pub config: &'a EventTableCommonConfig<F>,
    pub bit_index: i32,
    pub common_range_index: i32,
    pub unlimited_index: i32,
    pub u4_bop_index: i32,
    pub u64_index: i32,
    pub u64_on_u8_index: i32,
    pub mtable_lookup_index: i32,
    pub jtable_lookup_index: i32,
    pub pow_table_lookup_index: i32,
    pub offset_len_bits_lookup_index: i32,
}

impl<'a, F: FieldExt> EventTableCellAllocator<'a, F> {
    pub(super) fn new(config: &'a EventTableCommonConfig<F>) -> Self {
        Self {
            config,
            bit_index: EventTableBitColumnRotation::Max as i32,
            common_range_index: EventTableCommonRangeColumnRotation::Max as i32,
            unlimited_index: 0,
            u4_bop_index: 0,
            u64_index: 0,
            u64_on_u8_index: 0,
            pow_table_lookup_index: EventTableUnlimitColumnRotation::PowTableLookup as i32,
            mtable_lookup_index: EventTableUnlimitColumnRotation::MTableLookupStart as i32,
            jtable_lookup_index: EventTableUnlimitColumnRotation::JTableLookup as i32,
            offset_len_bits_lookup_index: EventTableUnlimitColumnRotation::OffsetLenBitsTableLookup
                as i32,
        }
    }

    pub fn alloc_bit_value(&mut self) -> BitCell {
        assert!(self.bit_index < BITS_COLUMNS as i32 * ETABLE_STEP_SIZE as i32);
        let allocated_index = self.bit_index;
        self.bit_index += 1;
        BitCell {
            col: self.config.shared_bits[allocated_index as usize / ETABLE_STEP_SIZE as usize],
            rot: allocated_index % ETABLE_STEP_SIZE as i32,
        }
    }

    pub fn alloc_common_range_value(&mut self) -> CommonRangeCell {
        assert!(self.common_range_index < ETABLE_STEP_SIZE as i32);
        let allocated_index = self.common_range_index;
        self.common_range_index += 1;
        CommonRangeCell {
            col: self.config.state,
            rot: allocated_index,
        }
    }

    pub fn alloc_unlimited_value(&mut self) -> UnlimitedCell {
        assert!(self.unlimited_index < ETABLE_STEP_SIZE as i32);
        let allocated_index = self.unlimited_index;
        self.unlimited_index += 1;
        UnlimitedCell {
            col: self.config.unlimited,
            rot: allocated_index,
        }
    }

    pub fn alloc_u4_bop(&mut self) -> U4BopCell {
        assert!(self.u4_bop_index < 1 as i32);
        self.u4_bop_index += 1;
        U4BopCell {
            col: self.config.u4_bop,
        }
    }

    pub fn alloc_u64(&mut self) -> U64Cell {
        assert!(self.u64_index < U4_COLUMNS as i32);
        let allocated_index = self.u64_index;
        self.u64_index += 1;
        U64Cell {
            value_col: self.config.aux,
            value_rot: allocated_index + EventTableUnlimitColumnRotation::U64Start as i32,
            u4_col: self.config.u4_shared[allocated_index as usize],
        }
    }

    pub fn alloc_u64_on_u8(&mut self) -> U64OnU8Cell {
        assert!(self.u64_on_u8_index < U8_COLUMNS as i32 * 2);
        let allocated_index = self.u64_on_u8_index;
        self.u64_on_u8_index += 1;
        U64OnU8Cell {
            value_col: self.config.aux,
            value_rot: allocated_index
                + EventTableUnlimitColumnRotation::U64Start as i32
                + U4_COLUMNS as i32,
            u8_col: self.config.u8_shared[allocated_index as usize / 2],
            u8_rot: (allocated_index % 2) * 8,
        }
    }

    pub fn alloc_mtable_lookup(&mut self) -> MTableLookupCell {
        assert!(self.mtable_lookup_index < EventTableUnlimitColumnRotation::U64Start as i32);
        let allocated_index = self.mtable_lookup_index;
        self.mtable_lookup_index += 1;
        MTableLookupCell {
            col: self.config.aux,
            rot: allocated_index,
        }
    }

    pub fn alloc_pow_table_lookup(&mut self) -> PowTableLookupCell {
        assert!(
            self.pow_table_lookup_index
                < EventTableUnlimitColumnRotation::OffsetLenBitsTableLookup as i32
        );
        let allocated_index = self.pow_table_lookup_index;
        self.pow_table_lookup_index += 1;
        PowTableLookupCell {
            col: self.config.aux,
            rot: allocated_index,
        }
    }

    pub fn alloc_offset_len_bits_table_lookup(&mut self) -> OffsetLenBitsTableLookupCell {
        assert!(
            self.offset_len_bits_lookup_index
                < EventTableUnlimitColumnRotation::MTableLookupStart as i32
        );
        let allocated_index = self.offset_len_bits_lookup_index;
        self.offset_len_bits_lookup_index += 1;
        OffsetLenBitsTableLookupCell {
            col: self.config.aux,
            rot: allocated_index,
        }
    }

    pub fn alloc_jtable_lookup(&mut self) -> JTableLookupCell {
        assert!(
            self.jtable_lookup_index < EventTableUnlimitColumnRotation::MTableLookupStart as i32
        );
        let allocated_index = self.jtable_lookup_index;
        self.jtable_lookup_index += 1;
        JTableLookupCell {
            col: self.config.aux,
            rot: allocated_index,
        }
    }
}

pub struct ConstraintBuilder<'a, F: FieldExt> {
    meta: &'a mut ConstraintSystem<F>,
    constraints: Vec<(
        &'static str,
        Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    )>,
    lookups: BTreeMap<
        &'static str,
        Vec<(
            &'static str,
            Box<dyn Fn(&mut VirtualCells<F>) -> Expression<F>>,
        )>,
    >,
}

impl<'a, F: FieldExt> ConstraintBuilder<'a, F> {
    pub(super) fn new(meta: &'a mut ConstraintSystem<F>) -> Self {
        Self {
            meta,
            constraints: vec![],
            lookups: BTreeMap::new(),
        }
    }

    pub fn push(
        &mut self,
        name: &'static str,
        builder: Box<dyn FnOnce(&mut VirtualCells<F>) -> Vec<Expression<F>>>,
    ) {
        self.constraints.push((name, builder));
    }

    pub fn lookup(
        &mut self,
        foreign_table_id: &'static str,
        name: &'static str,
        builder: Box<dyn Fn(&mut VirtualCells<F>) -> Expression<F>>,
    ) {
        match self.lookups.get_mut(&foreign_table_id) {
            Some(lookups) => lookups.push((name, builder)),
            None => {
                self.lookups.insert(foreign_table_id, vec![(name, builder)]);
            }
        }
    }

    pub(super) fn finalize(
        self,
        foreign_tables: &BTreeMap<&'static str, Box<dyn ForeignTableConfig<F>>>,
        enable: impl Fn(&mut VirtualCells<F>) -> Expression<F>,
    ) {
        for (name, builder) in self.constraints {
            self.meta.create_gate(&name, |meta| {
                builder(meta)
                    .into_iter()
                    .map(|constraint| constraint * enable(meta))
                    .collect::<Vec<_>>()
            });
        }

        for (id, lookups) in self.lookups {
            let config = foreign_tables.get(&id).unwrap();
            for (key, expr) in lookups {
                config.configure_in_table(self.meta, key, &|meta| expr(meta) * enable(meta));
            }
        }
    }
}

pub trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableForeignOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        common: &mut EventTableCellAllocator<F>,
        constraint_builder: &mut ConstraintBuilder<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn opcode_class(&self) -> OpcodeClass;

    fn assign(
        &self,
        ctx: &mut Context<'_, F>,
        step: &StepStatus,
        entry: &EventTableEntry,
    ) -> Result<(), Error>;

    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }

    fn jops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn next_last_jump_eid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn next_moid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn next_fid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn next_iid(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn mtable_lookup(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _item: MLookupItem,
        _common: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn jtable_lookup(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn itable_lookup(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn intable_lookup(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _common_config: &EventTableCommonConfig<F>,
    ) -> Option<Expression<F>> {
        None
    }
    fn is_host_input(&self) -> bool {
        false
    }
}
