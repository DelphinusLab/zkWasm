use std::fs::File;
use std::fs::OpenOptions;
use std::fs::{self};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use crate::config::CircuitDataConfig;
use crate::config::CircuitDataMd5;
use crate::TRIVIAL_WASM;
use clap::Args;
use console::style;
use delphinus_zkwasm::checksum::ImageCheckSum;
use delphinus_zkwasm::circuits::ZkWasmCircuit;
use delphinus_zkwasm::loader::ZkWasmLoader;
use delphinus_zkwasm::runtime::host::HostEnvBuilder;
use delphinus_zkwasm::runtime::monitor::table_monitor::TableMonitor;
use halo2_proofs::arithmetic::CurveAffine;
use halo2_proofs::pairing::bn256::Bn256;
use halo2_proofs::pairing::bn256::G1Affine;
use halo2_proofs::plonk::keygen_vk;
use halo2_proofs::plonk::Circuit;
use halo2_proofs::plonk::CircuitData;
use halo2_proofs::poly::commitment::Params;
use specs::slice::Slice;
use specs::CompilationTable;
use specs::TraceBackend;

use crate::args::HostMode;
use crate::config::Config;
use crate::names::name_of_circuit_data;
use crate::names::name_of_config;
use crate::names::name_of_params;

#[derive(Debug)]
pub(crate) struct SetupArg {
    pub(crate) k: u32,
    pub(crate) host_mode: HostMode,
    pub(crate) phantom_functions: Vec<String>,
    pub(crate) wasm_image: Option<PathBuf>,
}

impl SetupArg {
    fn _setup_circuit_data<C: CurveAffine, ConcreteCircuit: Circuit<C::Scalar>>(
        params: &Params<C>,
        circuit: &ConcreteCircuit,
        path_of_circuit_data: PathBuf,
    ) -> anyhow::Result<CircuitDataMd5> {
        let vkey = keygen_vk(params, circuit)?;
        let circuit_data = CircuitData::new(params, vkey, circuit)?;

        let verifying_key_md5 = {
            let mut buf = Vec::new();
            circuit_data.get_vkey().write(&mut buf)?;

            format!("{:x}", md5::compute(&buf))
        };

        let circuit_data_md5 = {
            circuit_data.write(
                &mut OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&path_of_circuit_data)?,
            )?;

            let mut buf = Vec::new();
            File::open(&path_of_circuit_data)?.read_to_end(&mut buf)?;

            format!("{:x}", md5::compute(&buf))
        };

        Ok(CircuitDataMd5 {
            circuit_data_md5,
            verifying_key_md5,
        })
    }

    pub(crate) fn setup_circuit_data(
        &self,
        name: &str,
        params_dir: &Path,
        params: &Params<G1Affine>,
        k: u32,
        compilation_tables: &CompilationTable,
    ) -> anyhow::Result<CircuitDataConfig> {
        let setup_circuit = |is_last_slice| -> anyhow::Result<CircuitDataMd5> {
            let setup_circuit = ZkWasmCircuit::new(
                k,
                Slice::from_compilation_table(compilation_tables, is_last_slice),
            )?;

            match setup_circuit {
                ZkWasmCircuit::Ongoing(circuit) => SetupArg::_setup_circuit_data(
                    params,
                    &circuit,
                    params_dir.join(name_of_circuit_data(name, is_last_slice)),
                ),
                ZkWasmCircuit::LastSliceCircuit(circuit) => SetupArg::_setup_circuit_data(
                    params,
                    &circuit,
                    params_dir.join(name_of_circuit_data(name, is_last_slice)),
                ),
            }
        };

        #[cfg(feature = "continuation")]
        return Ok(CircuitDataConfig {
            on_going_circuit: setup_circuit(false)?,
            finalized_circuit: setup_circuit(true)?,
        });

        #[cfg(not(feature = "continuation"))]
        return Ok(CircuitDataConfig {
            finalized_circuit: setup_circuit(true)?,
        });
    }

    pub(crate) fn setup(
        &self,
        env_builder: &dyn HostEnvBuilder,
        name: &str,
        params_dir: &PathBuf,
    ) -> anyhow::Result<()> {
        fs::create_dir_all(params_dir)?;

        let wasm_image = self.wasm_image.as_ref().map_or(
            wabt::wat2wasm(TRIVIAL_WASM).map_err(|err| anyhow::anyhow!(err)),
            |file| fs::read(file).map_err(|err| anyhow::anyhow!(err)),
        )?;
        let module = ZkWasmLoader::parse_module(&wasm_image)?;
        let wasm_image_md5 = md5::compute(&wasm_image);

        let params_path = params_dir.join(name_of_params(self.k));
        let params = {
            if params_path.exists() {
                println!(
                    "{} Found existing params at {:?}. Using it instead of building a new one...",
                    style("[1/5]").bold().dim(),
                    params_path.canonicalize()?
                );

                Params::<G1Affine>::read(&mut File::open(&params_path)?)?
            } else {
                println!(
                    "{} Building params for K = {}...",
                    style("[1/5]").bold().dim(),
                    self.k
                );
                let params = Params::<G1Affine>::unsafe_setup::<Bn256>(self.k);

                params.write(&mut File::create(&params_path)?)?;
                params
            }
        };

        let env = env_builder.create_env_without_value(self.k);
        let mut monitor =
            TableMonitor::new(self.k, &self.phantom_functions, TraceBackend::Memory, &env);

        let loader = ZkWasmLoader::new(self.k, env)?;

        println!("{} Compiling...", style("[2/5]").bold().dim());
        loader.compile(&module, &mut monitor)?;
        let compilation_table = monitor.into_compilation_table();

        println!("{} Building circuit data...", style("[3/5]").bold().dim(),);
        let circuit_datas =
            self.setup_circuit_data(name, params_dir, &params, self.k, &compilation_table)?;

        println!("{} Computing checksum...", style("[4/5]").bold().dim(),);
        let checksum = {
            let checksum = compilation_table.checksum(self.k, &params);
            assert_eq!(checksum.len(), 1);

            (checksum[0].x.to_string(), checksum[0].y.to_string())
        };

        {
            println!("{} Writing config...", style("[5/5]").bold().dim(),);

            let params_md5 = {
                let mut buf = Vec::new();

                params.write(&mut buf)?;
                let md5 = md5::compute(&buf);

                format!("{:x}", md5)
            };

            let config_path = params_dir.join(name_of_config(name));

            let config = Config {
                name: name.to_string(),

                k: self.k,
                params: params_path,
                is_uniform_circuit: cfg!(feature = "uniform-circuit"),

                params_md5,
                wasm_image_md5: if cfg!(feature = "uniform-circuit") {
                    None
                } else {
                    Some(format!("{:x}", wasm_image_md5))
                },
                circuit_datas,

                checksum,
                phantom_functions: self.phantom_functions.clone(),
                host_mode: self.host_mode,
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
}

#[derive(Debug, Args)]
pub(crate) struct RunningArg {
    /// Path to the directory to write the output.
    #[clap(short = 'o', long = "output")]
    pub(crate) output_dir: PathBuf,

    /// Public inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
    #[clap(long = "public")]
    pub(crate) public_inputs: Vec<String>,

    /// Private inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
    #[clap(long = "private")]
    pub(crate) private_inputs: Vec<String>,

    /// Context inputs with format 'value:type' where type=i64|bytes|bytes-packed|file.
    #[clap(long = "context-in")]
    pub(crate) context_inputs: Vec<String>,

    /// Filename to the file to write the context output.
    #[clap(long = "context-out")]
    pub(crate) context_output: Option<String>,
}

#[derive(Debug)]
pub(crate) struct DryRunArg {
    pub(crate) wasm_image: PathBuf,
    pub(crate) running_arg: RunningArg,
}

/// Execute the Wasm image and generate a proof.
#[derive(Debug)]
pub(crate) struct ProveArg {
    pub(crate) wasm_image: PathBuf,
    pub(crate) output_dir: PathBuf,
    pub(crate) running_arg: RunningArg,
    pub(crate) mock_test: bool,
    pub(crate) file_backend: bool,
    // skip first n slice(s) proving.
    pub(crate) skip: usize,
}

/// Verify the proof.
#[derive(Debug, Args)]
pub(crate) struct VerifyArg {
    /// Path to the directory to proof.
    #[clap(short = 'o', long = "output")]
    pub(crate) output_dir: PathBuf,
}

#[derive(Debug)]
pub(crate) enum Subcommands {
    Setup(SetupArg),
    DryRun(DryRunArg),
    Prove(ProveArg),
    Verify(VerifyArg),
}
