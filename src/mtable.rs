use halo2_proofs::arithmetic::FieldExt;
use halo2_proofs::plonk::Advice;
use halo2_proofs::plonk::Column;
use halo2_proofs::plonk::ConstraintSystem;
use halo2_proofs::plonk::Error;
use halo2_proofs::plonk::Expression;
use halo2_proofs::plonk::VirtualCells;
use halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

use crate::init_mtable::MInitTableConfig;
use crate::row_diff::RowDiffConfig;
use crate::rtable::RangeTableConfig;

pub enum LocationType {
    Heap,
    Stack,
}

impl<F: FieldExt> Into<Expression<F>> for LocationType {
    fn into(self) -> Expression<F> {
        match self {
            LocationType::Heap => Expression::Constant(F::from(0u64)),
            LocationType::Stack => Expression::Constant(F::from(1u64)),
        }
    }
}

pub enum AccessType {
    Read,
    Write,
    Init,
}

impl<F: FieldExt> Into<Expression<F>> for AccessType {
    fn into(self) -> Expression<F> {
        match self {
            AccessType::Read => Expression::Constant(F::from(1u64)),
            AccessType::Write => Expression::Constant(F::from(2u64)),
            AccessType::Init => Expression::Constant(F::from(3u64)),
        }
    }
}

pub enum VarType {
    U8,
    I32,
}

pub struct MemoryEvent {
    eid: u64,
    mmid: u64,
    offset: u64,
    ltype: LocationType,
    atype: AccessType,
    vtype: VarType,
    value: u64,
}

impl MemoryEvent {
    pub fn new(
        eid: u64,
        mmid: u64,
        offset: u64,
        ltype: LocationType,
        atype: AccessType,
        vtype: VarType,
        value: u64,
    ) -> MemoryEvent {
        MemoryEvent {
            eid,
            mmid,
            offset,
            ltype,
            atype,
            vtype,
            value,
        }
    }
}

pub struct MemoryTableConfig<F: FieldExt> {
    ltype: RowDiffConfig<F>,
    mmid: RowDiffConfig<F>,
    offset: RowDiffConfig<F>,
    eid: RowDiffConfig<F>,
    emid: Column<Advice>,
    atype: Column<Advice>,
    vtype: Column<Advice>,
    value: Column<Advice>,
    enable: Column<Advice>,
    same_location: Column<Advice>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> MemoryTableConfig<F> {
    fn new(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
    ) -> Self {
        let ltype = RowDiffConfig::configure("loc type", meta, cols);
        let mmid = RowDiffConfig::configure("mmid", meta, cols);
        let offset = RowDiffConfig::configure("mm offset", meta, cols);
        let eid = RowDiffConfig::configure("eid", meta, cols);
        let value = cols.next().unwrap();
        let atype = cols.next().unwrap();
        let vtype = cols.next().unwrap();
        let enable = cols.next().unwrap();
        let same_location = cols.next().unwrap();
        let emid = cols.next().unwrap();

        MemoryTableConfig {
            _mark: PhantomData,
            ltype,
            mmid,
            offset,
            eid,
            emid,
            atype,
            vtype,
            value,
            enable,
            same_location,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        cols: &mut impl Iterator<Item = Column<Advice>>,
        rtable: &RangeTableConfig<F>,
        minit_table: &MInitTableConfig<F>,
    ) -> Self {
        let mtconfig = Self::new(meta, cols);

        mtconfig.configure_enable(meta);
        mtconfig.configure_sort(meta, rtable);
        mtconfig.configure_stack_or_heap(meta);
        mtconfig.configure_range(meta, rtable);
        mtconfig.configure_same_location(meta);
        mtconfig.configure_rule(meta, minit_table);

        mtconfig
    }

    fn configure_enable(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("enable seq", |meta| {
            let curr = meta.query_advice(self.enable, Rotation::cur());
            let next = meta.query_advice(self.enable, Rotation::next());
            vec![
                next * (curr.clone() - Expression::<F>::Constant(F::one())),
                curr.clone() * (curr.clone() - Expression::<F>::Constant(F::one())),
            ]
        });
    }

    fn configure_same_location(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("is same location", |meta| {
            let same_loc = meta.query_advice(self.same_location, Rotation::cur());
            vec![
                self.ltype.is_same(meta) * self.mmid.is_same(meta) * self.offset.is_same(meta)
                    - same_loc,
            ]
        })
    }

    fn configure_stack_or_heap(&self, meta: &mut ConstraintSystem<F>) {
        meta.create_gate("is same location", |meta| {
            let ltype = self.ltype.data(meta);
            vec![ltype.clone() * (ltype - Expression::Constant(F::one()))]
        })
    }

    fn configure_range(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        rtable.configure_in_range(meta, "mmid in range", |meta| self.mmid.data(meta));

        rtable.configure_in_range(meta, "offset in range", |meta| self.offset.data(meta));

        rtable.configure_in_range(meta, "eid in range", |meta| self.eid.data(meta));

        rtable.configure_in_range(meta, "emid in range", |meta| {
            meta.query_advice(self.emid, Rotation::cur())
        });

        rtable.configure_in_range(meta, "vtype in range", |meta| {
            meta.query_advice(self.emid, Rotation::cur())
        });
    }

    fn configure_sort(&self, meta: &mut ConstraintSystem<F>, rtable: &RangeTableConfig<F>) {
        rtable.configure_in_range(meta, "ltype sort", |meta| {
            self.is_enable(meta) * self.ltype.diff(meta)
        });

        rtable.configure_in_range(meta, "mmid sort", |meta| {
            self.is_enable(meta) * self.ltype.is_same(meta) * self.mmid.diff(meta)
        });
        rtable.configure_in_range(meta, "offset sort", |meta| {
            self.is_enable(meta)
                * self.ltype.is_same(meta)
                * self.mmid.is_same(meta)
                * self.offset.diff(meta)
        });
        rtable.configure_in_range(meta, "eid sort", |meta| {
            self.is_enable(meta) * self.is_same_location(meta) * self.eid.diff(meta)
        });
        rtable.configure_in_range(meta, "emid sort", |meta| {
            self.is_enable(meta)
                * self.is_same_location(meta)
                * self.eid.is_same(meta)
                * (meta.query_advice(self.emid, Rotation::cur())
                    - meta.query_advice(self.emid, Rotation::prev()))
        });
    }

    fn configure_rule(&self, meta: &mut ConstraintSystem<F>, minit_table: &MInitTableConfig<F>) {
        meta.create_gate("read after write", |meta| {
            vec![
                self.is_enable(meta) * self.is_read_not_bit(meta) * self.diff(meta, self.value),
                self.is_enable(meta) * self.is_read_not_bit(meta) * self.diff(meta, self.vtype),
            ]
        });

        meta.create_gate("stack first line", |meta| {
            vec![
                self.is_enable(meta)
                    * (self.is_same_location(meta) - Expression::Constant(F::one()))
                    * self.is_stack(meta)
                    * (meta.query_advice(self.atype, Rotation::cur()) - AccessType::Write.into()),
            ]
        });

        // heap first line
        minit_table.configure_in_range(meta, "heap first line", |meta| {
            self.is_enable(meta)
                * (Expression::Constant(F::one()) - self.is_same_location(meta))
                * self.is_heap(meta)
                * minit_table.encode(
                    self.mmid.data(meta),
                    self.offset.data(meta),
                    meta.query_advice(self.value, Rotation::cur()),
                )
        })
    }

    fn is_heap(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        Expression::Constant(F::one()) - self.ltype.data(meta)
    }

    fn is_stack(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        self.ltype.data(meta)
    }

    fn diff(&self, meta: &mut VirtualCells<F>, col: Column<Advice>) -> Expression<F> {
        meta.query_advice(col, Rotation::cur()) - meta.query_advice(col, Rotation::prev())
    }

    fn is_enable(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        meta.query_advice(self.enable, Rotation::cur())
    }

    fn is_same_location(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        meta.query_advice(self.same_location, Rotation::cur())
    }

    fn is_read_not_bit(&self, meta: &mut VirtualCells<F>) -> Expression<F> {
        let atype = meta.query_advice(self.atype, Rotation::cur());
        (atype.clone() - AccessType::Init.into()) * (atype - AccessType::Write.into())
    }
}

pub struct MemoryTableChip<F: FieldExt> {
    config: MemoryTableConfig<F>,
    _phantom: PhantomData<F>,
}

impl<F: FieldExt> MemoryTableChip<F> {
    pub fn new(config: MemoryTableConfig<F>) -> Self {
        MemoryTableChip {
            config,
            _phantom: PhantomData,
        }
    }
}
