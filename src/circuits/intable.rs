use super::{config::MAX_INTABLE_ROWS, FromBn};
use crate::{
    fixed_curr,
    foreign::{ForeignCallInfo, ForeignTableConfig},
    instance_curr,
};
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::Layouter,
    plonk::{Column, ConstraintSystem, Error, Expression, Fixed, Instance, VirtualCells},
};
use num_bigint::BigUint;
use specs::{host_function::HostPlugin, itable::OpcodeClass};
use std::{
    marker::PhantomData,
    ops::{Add, Mul},
};

pub const INPUT_TABLE_KEY: &'static str = "input-table";

pub struct InputTableEncode {}

impl InputTableEncode {
    pub(super) fn encode_for_lookup<T: FromBn + Add<T, Output = T> + Mul<T, Output = T>>(
        index: T,
        input: T,
    ) -> T {
        T::from_bn(&(BigUint::from(1u64) << 128))
            + index * T::from_bn(&(BigUint::from(1u64) << 64))
            + input
    }
}

#[derive(Clone)]
pub struct InputTableConfig<F: FieldExt> {
    enable: Column<Fixed>,
    index: Column<Fixed>,
    // TODO: constrain input to u64
    input: Column<Instance>,
    _mark: PhantomData<F>,
}

impl<F: FieldExt> InputTableConfig<F> {
    fn new(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            enable: meta.fixed_column(),
            index: meta.fixed_column(),
            input: meta.instance_column(),
            _mark: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let config = Self::new(meta);

        config
    }

    pub fn encode(&self, meta: &mut VirtualCells<'_, F>) -> Expression<F> {
        fixed_curr!(meta, self.enable)
            * InputTableEncode::encode_for_lookup(
                fixed_curr!(meta, self.index),
                instance_curr!(meta, self.input),
            )
    }
}

impl<F: FieldExt> ForeignTableConfig<F> for InputTableConfig<F> {
    fn configure_in_table(
        &self,
        meta: &mut ConstraintSystem<F>,
        key: &'static str,
        expr: &dyn Fn(&mut VirtualCells<'_, F>) -> Expression<F>,
    ) {
        meta.lookup_any(key, |meta| vec![(expr(meta), self.encode(meta))]);
    }
}

pub struct InputTableChip<F: FieldExt> {
    config: InputTableConfig<F>,
}

impl<F: FieldExt> InputTableChip<F> {
    pub fn new(config: InputTableConfig<F>) -> Self {
        InputTableChip { config }
    }

    pub fn assign(self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_region(
            || "input table",
            |mut meta| {
                for i in 0..MAX_INTABLE_ROWS {
                    meta.assign_fixed(
                        || "input table enable",
                        self.config.enable,
                        i,
                        || Ok(F::one()),
                    )?;
                    meta.assign_fixed(
                        || "input table index",
                        self.config.index,
                        i,
                        || Ok(F::from(i as u64)),
                    )?;
                }

                Ok(())
            },
        )?;
        Ok(())
    }

    pub fn assign_public_input() -> Result<(), Error> {
        Ok(())
    }
}

pub struct InputForeignCallInfo {}
impl ForeignCallInfo for InputForeignCallInfo {
    fn call_id(&self) -> usize {
        OpcodeClass::ForeignPluginStart as usize + HostPlugin::HostInput as usize
    }
}
