use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::Fq;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::get_advice_commitments_from_transcript;
use halo2_proofs::plonk::VerifyingKey;
use halo2_proofs::poly::commitment::Params;
use halo2aggregator_s::transcript::poseidon::PoseidonRead;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use std::io;

pub fn get_named_advice_commitment(
    vkey: &VerifyingKey<G1Affine>,
    proof: &[u8],
    named_advice: &str,
) -> G1Affine {
    let img_col_idx = vkey
        .cs
        .named_advices
        .iter()
        .find(|(k, _)| k == named_advice)
        .unwrap()
        .1;

    get_advice_commitments_from_transcript::<Bn256, _, _>(vkey, &mut PoseidonRead::init(proof))
        .unwrap()[img_col_idx as usize]
}

pub(crate) trait WriteUncomressed {
    fn write_uncompressed<W: io::Write>(&self, writer: &mut W) -> io::Result<()>;
    fn read_uncompressed<R: io::Read>(reader: R) -> io::Result<Self>
    where
        Self: Sized;
}

impl WriteUncomressed for Params<G1Affine> {
    fn write_uncompressed<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.k.to_le_bytes())?;
        for el in &self.g {
            for i in 0..4 {
                writer.write_all(&el.x.0[i].to_le_bytes())?;
            }
            for i in 0..4 {
                writer.write_all(&el.y.0[i].to_le_bytes())?;
            }
        }
        for el in &self.g_lagrange {
            for i in 0..4 {
                writer.write_all(&el.x.0[i].to_le_bytes())?;
            }
            for i in 0..4 {
                writer.write_all(&el.y.0[i].to_le_bytes())?;
            }
        }
        let additional_data_len = self.additional_data.len() as u32;
        writer.write_all(&additional_data_len.to_le_bytes())?;
        writer.write_all(&self.additional_data)?;
        Ok(())
    }

    /// Reads params from a buffer.
    fn read_uncompressed<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let mut k = [0u8; 4];
        reader.read_exact(&mut k[..])?;
        let k = u32::from_le_bytes(k);
        let n = 1 << k;

        let load_points_from_file_parallelly = |reader: &mut R| -> io::Result<Vec<G1Affine>> {
            let mut points_bytes: Vec<[u8; 8]> = vec![[0u8; 8]; n * 2 * 4];
            for points in points_bytes.iter_mut() {
                reader.read_exact((*points).as_mut())?;
            }
            let mut points = vec![G1Affine::default(); n];
            points.par_iter_mut().enumerate().for_each(|(i, point)| {
                *point = G1Affine {
                    x: Fq([
                        u64::from_le_bytes(points_bytes[i * 8]),
                        u64::from_le_bytes(points_bytes[i * 8 + 1]),
                        u64::from_le_bytes(points_bytes[i * 8 + 2]),
                        u64::from_le_bytes(points_bytes[i * 8 + 3]),
                    ]),
                    y: Fq([
                        u64::from_le_bytes(points_bytes[i * 8 + 4]),
                        u64::from_le_bytes(points_bytes[i * 8 + 5]),
                        u64::from_le_bytes(points_bytes[i * 8 + 6]),
                        u64::from_le_bytes(points_bytes[i * 8 + 7]),
                    ]),
                }
            });
            Ok(points)
        };

        let g = load_points_from_file_parallelly(&mut reader)?;
        let g_lagrange = load_points_from_file_parallelly(&mut reader)?;

        let mut additional_data_len = [0u8; 4];
        reader.read_exact(&mut additional_data_len[..])?;
        let additional_data_len = u32::from_le_bytes(additional_data_len);
        let mut additional_data = vec![0u8; additional_data_len as usize];

        reader.read_exact(&mut additional_data[..])?;

        Ok(Params {
            k,
            n: n as u64,
            g,
            g_lagrange,
            additional_data,
        })
    }
}
