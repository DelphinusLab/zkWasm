use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use circuits_batcher::proof::CircuitInfo;
use circuits_batcher::proof::ProofInfo;
use circuits_batcher::proof::ProofLoadInfo;
use console::style;
use delphinus_zkwasm::circuits::TestCircuit;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::HostEnvArg;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::Fr;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::CircuitData;
use halo2_proofs::poly::commitment::Params;
use halo2aggregator_s::circuits::utils::TranscriptHash;
use halo2aggregator_s::native_verifier;
use serde::Deserialize;
use serde::Serialize;

use crate::args::HostMode;
use crate::names::name_of_circuit_data;
use crate::names::name_of_config;
use crate::names::name_of_loadinfo;
use crate::names::name_of_params;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    name: String,

    is_uniform_circuit: bool,
    k: u32,
    params: PathBuf,

    params_md5: String,
    circuit_data_md5: String,
    verifying_key_md5: String,
    wasm_image_md5: Option<String>,

    checksum: (String, String),
    phantom_functions: Vec<String>,
    pub(crate) host_mode: HostMode,
}

impl Config {
    fn image_consistent_check(&self, wasm_image: &[u8]) -> anyhow::Result<()> {
        if let Some(expected_wasm_image_md5) = &self.wasm_image_md5 {
            let wasm_image_md5 = format!("{:x}", md5::compute(wasm_image));

            if expected_wasm_image_md5 != &wasm_image_md5 {
                anyhow::bail!(
                    "Wasm image is inconsistent with the one used to build the circuit. \
                        Maybe you have changed the Wasm image after setup the circuit?",
                );
            }
        }

        Ok(())
    }

    fn params_consistent_check(&self, params: &[u8]) -> anyhow::Result<()> {
        let params_md5 = format!("{:x}", md5::compute(params));

        if params_md5 != self.params_md5 {
            anyhow::bail!(
                "Params is inconsistent with the one used to build the circuit. \
                    Maybe you have changed the params after setup the circuit?",
            );
        }

        Ok(())
    }

    fn circuit_data_consistent_check(&self, circuit_data: &[u8]) -> anyhow::Result<()> {
        let circuit_data_md5 = format!("{:x}", md5::compute(&circuit_data));

        if circuit_data_md5 != self.circuit_data_md5 {
            anyhow::bail!(
                "Circuit data is inconsistent with the one used to build the circuit. \
                    Maybe you have changed the circuit data after setup the circuit?",
            );
        }

        Ok(())
    }

    fn veryfying_key_consistent_check(&self, verifying_key: &[u8]) -> anyhow::Result<()> {
        let verifying_key_md5 = format!("{:x}", md5::compute(&verifying_key));

        if verifying_key_md5 != self.verifying_key_md5 {
            anyhow::bail!(
                "Verifying key is inconsistent with the one used to build the circuit. \
                    Maybe you have changed the circuit data after setup the circuit?",
            );
        }

        Ok(())
    }
}

impl Config {
    fn write(&self, fd: &mut File) -> anyhow::Result<()> {
        fd.write(&bincode::serialize(self)?)?;

        Ok(())
    }

    pub(crate) fn read(fd: &mut File) -> anyhow::Result<Self> {
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf)?;
        let config = bincode::deserialize(&mut buf)?;

        Ok(config)
    }
}

impl Config {
    fn read_wasm_image(&self, wasm_image: &PathBuf) -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        File::open(&wasm_image)?.read_to_end(&mut buf)?;

        self.image_consistent_check(&buf)?;

        Ok(buf)
    }

    fn read_params(&self, params_dir: &PathBuf) -> anyhow::Result<Params<G1Affine>> {
        let path = params_dir.join(name_of_params(self.k));

        let mut buf = Vec::new();
        File::open(&path)?.read_to_end(&mut buf)?;

        self.params_consistent_check(&buf)?;

        let params = Params::<G1Affine>::read(&mut Cursor::new(&mut buf))?;

        Ok(params)
    }

    fn read_circuit_data(
        &self,
        params_dir: &PathBuf,
        file: &str,
    ) -> anyhow::Result<CircuitData<G1Affine>> {
        let path = params_dir.join(file);

        let mut buf = Vec::new();
        File::open(&path)?.read_to_end(&mut buf)?;

        self.circuit_data_consistent_check(&buf)?;

        let circuit_data = CircuitData::<G1Affine>::read(&mut File::open(&path)?)?;

        Ok(circuit_data)
    }

    pub(crate) fn setup<EnvBuilder: HostEnvBuilder>(
        name: &str,
        k: u32,
        wasm_image: Vec<u8>,
        phantom_functions: Vec<String>,
        host_mode: HostMode,
        params_dir: &PathBuf,
    ) -> Result<()> {
        fs::create_dir_all(params_dir)?;

        let params_path = params_dir.join(name_of_params(k));
        let params = {
            if params_path.exists() {
                println!(
                    "{} Found existing params at {:?}. Using it instead of building a new one...",
                    style("[1/4]").bold().dim(),
                    params_path.canonicalize()?
                );

                Params::<G1Affine>::read(&mut File::open(&params_path)?)?
            } else {
                println!(
                    "{} Building params for K = {}...",
                    style("[1/4]").bold().dim(),
                    k
                );
                let params = Params::<G1Affine>::unsafe_setup::<Bn256>(k);

                params.write(&mut File::create(&params_path)?)?;
                params
            }
        };

        let loader = ZkWasmLoader::<Bn256, _, EnvBuilder>::new(
            k,
            wasm_image.clone(),
            phantom_functions.clone(),
        )?;

        println!("{} Building circuit data...", style("[2/4]").bold().dim(),);
        let (verifying_key_md5, circuit_data_md5) = {
            let circuit = loader.circuit_without_witness(EnvBuilder::HostConfig::default())?;
            let vkey = loader.create_vkey(&params, &circuit)?;
            let circuit_data = CircuitData::new(&params, vkey, &circuit)?;

            let verifying_key_md5 = {
                let mut buf = Vec::new();
                circuit_data.get_vkey().write(&mut buf)?;

                format!("{:x}", md5::compute(&buf))
            };

            let circuit_data_md5 = {
                let path = params_dir.join(name_of_circuit_data(name));

                circuit_data.write(
                    &mut OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(&path)?,
                )?;

                let mut buf = Vec::new();
                File::open(&path)?.read_to_end(&mut buf)?;

                format!("{:x}", md5::compute(&buf))
            };

            (verifying_key_md5, circuit_data_md5)
        };

        println!("{} Computing checksum...", style("[3/4]").bold().dim(),);
        let checksum = {
            let checksum = loader.checksum(&params, EnvBuilder::HostConfig::default())?;
            assert_eq!(checksum.len(), 1);

            (checksum[0].x.to_string(), checksum[0].y.to_string())
        };

        {
            println!("{} Writing config...", style("[4/4]").bold().dim(),);

            let params_md5 = {
                let mut buf = Vec::new();

                params.write(&mut buf)?;
                let md5 = md5::compute(&buf);

                format!("{:x}", md5)
            };

            let config_path = params_dir.join(&name_of_config(name));

            let config = Config {
                name: name.to_string(),

                k,
                params: params_path,
                is_uniform_circuit: cfg!(feature = "uniform-circuit"),

                params_md5,
                circuit_data_md5,
                verifying_key_md5,
                wasm_image_md5: if cfg!(feature = "uniform-circuit") {
                    None
                } else {
                    Some(format!("{:x}", md5::compute(&wasm_image)))
                },

                checksum,
                phantom_functions,
                host_mode,
            };
            config.write(&mut File::create(&config_path)?)?;

            println!(
                "{} {:?}",
                style("The configuration is saved at").green().bold().dim(),
                config_path.canonicalize()?
            );
        }

        Ok(())
    }

    pub(crate) fn dry_run<EnvBuilder: HostEnvBuilder>(
        self,
        wasm_image: &PathBuf,
        output_dir: &PathBuf,
        arg: EnvBuilder::Arg,
        context_output_filename: Option<String>,
    ) -> Result<()> {
        let wasm_image = self.read_wasm_image(wasm_image)?;

        let context_output = arg.get_context_output();

        let loader = ZkWasmLoader::<Bn256, EnvBuilder::Arg, EnvBuilder>::new(
            self.k,
            wasm_image,
            self.phantom_functions,
        )?;

        {
            println!("{} Executing...", style("[1/2]").bold().dim(),);
            let result = loader.run(arg, EnvBuilder::HostConfig::default(), true)?;

            println!("total guest instructions used {:?}", result.guest_statics);
            println!("total host api used {:?}", result.host_statics);
        };

        {
            if let Some(context_output_filename) = context_output_filename {
                let context_output_path = output_dir.join(context_output_filename);

                println!(
                    "{} Write context output to file {:?}...",
                    style("[2/2]").bold().dim(),
                    context_output_path
                );

                context_output.write(&mut File::create(&context_output_path)?)?;
            } else {
                println!(
                    "{} Context output is not specified. Skip writing context output.",
                    style("[2/2]").bold().dim()
                );
            }
        }

        Ok(())
    }

    pub(crate) fn prove<EnvBuilder: HostEnvBuilder>(
        self,
        wasm_image: &PathBuf,
        params_dir: &PathBuf,
        output_dir: &PathBuf,
        arg: EnvBuilder::Arg,
        context_output_filename: Option<String>,
        mock_test: bool,
    ) -> anyhow::Result<()> {
        println!("{} Prepare...", style("[1/8]").bold().dim(),);

        let params = self.read_params(params_dir)?;
        let proving_key = self
            .read_circuit_data(params_dir, &name_of_circuit_data(&self.name))?
            .into_proving_key(&params);
        let wasm_image = self.read_wasm_image(wasm_image)?;

        let context_output = arg.get_context_output();

        let loader = ZkWasmLoader::<Bn256, EnvBuilder::Arg, EnvBuilder>::new(
            self.k,
            wasm_image,
            self.phantom_functions,
        )?;

        let result = {
            println!("{} Executing...", style("[2/8]").bold().dim(),);
            let result = loader.run(arg, EnvBuilder::HostConfig::default(), false)?;

            println!("total guest instructions used {:?}", result.guest_statics);
            println!("total host api used {:?}", result.host_statics);

            result
        };

        {
            if let Some(context_output_filename) = context_output_filename {
                let context_output_path = output_dir.join(context_output_filename);

                println!(
                    "{} Write context output to file {:?}...",
                    style("[3/8]").bold().dim(),
                    context_output_path
                );

                context_output.write(&mut File::create(&context_output_path)?)?;
            } else {
                println!(
                    "{} Context output is not specified. Skip writing context output.",
                    style("[3/8]").bold().dim()
                );
            }
        }

        {
            let dir = output_dir.join("traces");
            fs::create_dir_all(&dir)?;

            println!(
                "{} Writing traces to {:?}...",
                style("[4/8]").bold().dim(),
                dir
            );
            result.tables.write(&dir);
        }

        let (circuit, instances) = {
            println!("{} Build circuit...", style("[5/8]").bold().dim(),);

            loader.circuit_with_witness(result)?
        };

        {
            if mock_test {
                println!("{} Mock test...", style("[6/8]").bold().dim(),);
                loader.mock_test(&circuit, &instances)?;
            } else {
                println!("{} Mock test is skipped", style("[6/8]").bold().dim(),);
            }
        }

        {
            let circuit: CircuitInfo<Bn256, TestCircuit<Fr>> = CircuitInfo::new(
                circuit,
                self.name.clone(),
                vec![instances],
                self.k as usize,
                circuits_batcher::args::HashType::Poseidon,
            );

            println!("{} Creating proof...", style("[7/8]").bold().dim(),);
            circuit.exec_create_proof_with_params(&params, &proving_key, output_dir, 0);

            let proof_load_info_path = output_dir.join(&name_of_loadinfo(&self.name));
            println!(
                "{} Saving proof load info at {:?}...",
                style("[8/8]").bold().dim(),
                proof_load_info_path
            );
            circuit
                .proofloadinfo
                .save(proof_load_info_path.parent().unwrap());
        }

        Ok(())
    }

    pub(crate) fn verify(self, params_dir: &PathBuf, output_dir: &PathBuf) -> anyhow::Result<()> {
        let proof = {
            println!(
                "{} Reading proof from {:?}",
                style("[1/4]").bold().dim(),
                output_dir
            );

            let proof_load_info =
                ProofLoadInfo::load(&output_dir.join(&name_of_loadinfo(&self.name)));

            let proofs: Vec<ProofInfo<Bn256>> =
                ProofInfo::load_proof(&output_dir, &params_dir, &proof_load_info);

            assert_eq!(proofs.len(), 1, "zkWasm cli doesn't handle multiple proofs");

            proofs.into_iter().nth(0).unwrap()
        };

        let params_verifier = {
            println!(
                "{} Building verifier params...",
                style("[3/4]").bold().dim(),
            );

            let public_inputs_size = proof
                .instances
                .iter()
                .fold(0, |acc, x| usize::max(acc, x.len()));

            let params = self.read_params(params_dir)?;
            params.verifier(public_inputs_size)?
        };

        {
            let mut buf = Vec::new();
            proof.vkey.write(&mut Cursor::new(&mut buf))?;

            self.veryfying_key_consistent_check(&buf)?;
        };

        {
            println!("{} Verifying...", style("[3/3]").bold().dim(),);

            native_verifier::verify_single_proof::<Bn256>(
                &params_verifier,
                &proof.vkey,
                &proof.instances,
                proof.transcripts,
                TranscriptHash::Poseidon,
            );
        }

        println!("{}", style("Verification succeeded!").green().bold().dim(),);

        Ok(())
    }
}
