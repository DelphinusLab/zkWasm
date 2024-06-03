use std::collections::HashMap;

use halo2_proofs::arithmetic::FieldExt;
use specs::configure_table::ConfigureTable;
use specs::itable::InstructionTable;

#[derive(Clone)]
pub struct Status<'a> {
    pub eid: u32,
    pub fid: u32,
    pub iid: u32,
    pub sp: u32,
    pub last_jump_eid: u32,
    pub allocated_memory_pages: u32,

    pub rest_mops: u32,
    pub rest_call_ops: u32,
    pub rest_return_ops: u32,

    pub host_public_inputs: u32,
    pub context_in_index: u32,
    pub context_out_index: u32,
    pub external_host_call_call_index: u32,

    pub itable: &'a InstructionTable,
}

#[derive(Default)]
pub struct FieldHelper<F: FieldExt>(HashMap<u64, F>);

impl<F: FieldExt> FieldHelper<F> {
    pub fn invert(&mut self, value: u64) -> F {
        *self
            .0
            .entry(value)
            .or_insert_with(|| F::from(value).invert().unwrap_or(F::zero()))
    }
}

pub struct StepStatus<'a, 'b, 'c, 'd, F: FieldExt> {
    pub current: &'a Status<'b>,
    pub next: &'a Status<'b>,
    pub configure_table: &'b ConfigureTable,
    pub frame_table_returned_lookup: &'c HashMap<(u32, u32), bool>,
    pub field_helper: &'d mut FieldHelper<F>,
}
