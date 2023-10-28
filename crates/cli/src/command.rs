use clap::App;
use clap::Command;

use super::args::ArgBuilder;

pub trait CommandBuilder: ArgBuilder {
    fn append_setup_subcommand(app: App) -> App {
        let command = Command::new("setup");

        app.subcommand(command)
    }

    fn append_dry_run_subcommand(app: App) -> App {
        let command = Command::new("dry-run")
            .arg(Self::single_public_arg())
            .arg(Self::single_private_arg())
            .arg(Self::dry_run_service_arg())
            .arg(Self::context_in_arg())
            .arg(Self::context_out_path_arg());

        app.subcommand(command)
    }

    fn append_create_single_proof_subcommand(app: App) -> App {
        let command = Command::new("single-prove")
            .arg(Self::single_public_arg())
            .arg(Self::single_private_arg())
            .arg(Self::context_in_arg())
            .arg(Self::context_out_path_arg());

        app.subcommand(command)
    }

    fn append_verify_single_proof_subcommand(app: App) -> App {
        let command = Command::new("single-verify")
            .arg(Self::proof_path_arg())
            .arg(Self::single_instance_path_arg());

        app.subcommand(command)
    }

    fn append_create_aggregate_proof_subcommand(app: App) -> App {
        let command = Command::new("aggregate-prove")
            .arg(Self::aggregate_public_args())
            .arg(Self::aggregate_private_args());

        app.subcommand(command)
    }

    fn append_verify_aggregate_verify_subcommand(app: App) -> App {
        let command = Command::new("aggregate-verify")
            .arg(Self::proof_path_arg())
            .arg(Self::instances_path_arg());

        app.subcommand(command)
    }

    fn append_generate_solidity_verifier(app: App) -> App {
        let command = Command::new("solidity-aggregate-verifier")
            .arg(Self::sol_dir_arg())
            .arg(Self::proof_path_arg())
            .arg(Self::auxonly_arg())
            .arg(Self::instances_path_arg());

        app.subcommand(command)
    }
}
