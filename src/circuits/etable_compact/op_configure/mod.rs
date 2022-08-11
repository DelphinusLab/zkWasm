use super::*;
use halo2_proofs::{arithmetic::FieldExt, plonk::ConstraintSystem};

pub mod op_const;
pub mod op_return;

pub struct Cell {
    pub col: Column<Advice>,
    pub rot: i32,
}

pub struct MTableLookupCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

pub struct BitCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

pub struct CommonRangeCell {
    pub col: Column<Advice>,
    pub rot: i32,
}

impl CommonRangeCell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, _value: u16) -> Result<(), Error> {
        todo!()
    }
}

pub struct U64Cell {
    pub value_col: Column<Advice>,
    pub value_rot: i32,
    pub u4_col: Column<Advice>,
}

impl U64Cell {
    pub fn assign<F: FieldExt>(&self, ctx: &mut Context<'_, F>, _value: u64) -> Result<(), Error> {
        todo!()
    }
}

pub struct EventTableCellAllocator<'a, F> {
    pub config: &'a EventTableCommonConfig<F>,
    pub bit_index: i32,
    pub common_range_index: i32,
    pub unlimit_index: i32,
    pub u64_index: i32,
    pub mtable_lookup_index: i32,
}

impl<'a, F: FieldExt> EventTableCellAllocator<'a, F> {
    pub fn new(config: &'a EventTableCommonConfig<F>) -> Self {
        Self {
            config,
            bit_index: EventTableBitColumnRotation::Max as i32,
            common_range_index: EventTableCommonRangeColumnRotation::Max as i32,
            unlimit_index: EventTableUnlimitColumnRotation::SharedStart as i32,
            u64_index: 0,
            mtable_lookup_index: EventTableUnlimitColumnRotation::MTableLookupStart as i32,
        }
    }

    pub fn alloc_bit_value(&mut self) -> BitCell {
        assert!(self.bit_index < ETABLE_STEP_SIZE as i32);
        let allocated_index = self.bit_index;
        self.bit_index += 1;
        BitCell {
            col: self.config.shared_bits,
            rot: allocated_index,
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

    pub fn alloc_unlimited_value(&mut self) -> Cell {
        assert!(self.unlimit_index < ETABLE_STEP_SIZE as i32);
        let allocated_index = self.unlimit_index;
        self.unlimit_index += 1;
        Cell {
            col: self.config.aux,
            rot: allocated_index,
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

    pub fn alloc_mtable_lookup(&mut self) -> MTableLookupCell {
        assert!(self.mtable_lookup_index < EventTableUnlimitColumnRotation::U64Start as i32);
        let allocated_index = self.mtable_lookup_index;
        self.mtable_lookup_index += 1;
        MTableLookupCell {
            col: self.config.aux,
            rot: allocated_index,
        }
    }
}

pub(super) trait EventTableOpcodeConfigBuilder<F: FieldExt> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        common: &mut EventTableCellAllocator<F>,
        enable: impl Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) -> Box<dyn EventTableOpcodeConfig<F>>;
}

pub(super) trait EventTableOpcodeConfig<F: FieldExt> {
    fn opcode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F>;
    fn assign(&self, ctx: &mut Context<'_, F>, entry: &EventTableEntry) -> Result<(), Error>;
    fn opcode_class(&self) -> OpcodeClass;
    fn jops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn mops(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn next_last_jump_eid(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn next_moid(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn next_fid(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn next_iid(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn sp_diff(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn mtable_lookup(
        &self,
        _meta: &mut VirtualCells<'_, F>,
        _mtable: &MemoryTableConfig<F>,
        _item: MLookupItem,
    ) -> Option<Expression<F>> {
        None
    }
    fn jtable_lookup(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
    fn itable_lookup(&self, _meta: &mut VirtualCells<'_, F>) -> Option<Expression<F>> {
        None
    }
}
