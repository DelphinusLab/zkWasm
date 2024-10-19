use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::get_advice_commitments_from_transcript;
use halo2_proofs::plonk::VerifyingKey;
use halo2aggregator_s::transcript::poseidon::PoseidonRead;

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
