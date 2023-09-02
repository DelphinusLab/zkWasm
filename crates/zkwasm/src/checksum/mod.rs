use halo2_proofs::arithmetic::best_multiexp_gpu_cond;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::poly::commitment::Params;
use specs::CompilationTable;

use crate::circuits::image_table::EncodeCompilationTableValues;

pub trait ImageCheckSum<Output> {
    fn checksum(&self) -> Output;
}

pub(crate) struct CompilationTableWithParams<'a, 'b, C: CurveAffine> {
    pub(crate) table: &'a CompilationTable,
    pub(crate) params: &'b Params<C>,
}

impl<'a, 'b, C: CurveAffine> ImageCheckSum<Vec<C>> for CompilationTableWithParams<'a, 'b, C> {
    fn checksum(&self) -> Vec<C> {
        let cells = self.table.encode_compilation_table_values().plain();

        let c = best_multiexp_gpu_cond(&cells[..], &self.params.get_g_lagrange()[0..cells.len()]);
        vec![c.into()]
    }
}
