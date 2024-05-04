use halo2_proofs::arithmetic::best_multiexp_gpu_cond;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::poly::commitment::Params;
use specs::CompilationTable;

use crate::circuits::utils::image_table::encode_compilation_table_values;

pub trait ImageCheckSum<C: CurveAffine, Output> {
    fn checksum(&self, k: u32, params: &Params<C>) -> Output;
}

impl<C: CurveAffine> ImageCheckSum<C, Vec<C>> for CompilationTable {
    fn checksum(&self, k: u32, params: &Params<C>) -> Vec<C> {
        let cells = encode_compilation_table_values(
            k,
            &self.itable,
            &self.br_table,
            &self.elem_table,
            &self.initial_frame_table,
            &self.initialization_state,
            &self.imtable,
        )
        .plain();

        let c = best_multiexp_gpu_cond(&cells[..], &params.get_g_lagrange()[0..cells.len()]);
        vec![c.into()]
    }
}
