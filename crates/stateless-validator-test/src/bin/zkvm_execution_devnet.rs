//! Runs the latest blocks published in the devnet catalog through a
//! release-backed stateless validator guest in a zkVM, reporting failures.
//!
//! Run with env `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use the pre-built
//! image as the executor.

use std::{fs, path::PathBuf};

use clap::Parser;
use ere_dockerized::zkVMKind;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{
        ExecutionFailures, init_tracing,
        zkvm::{is_guest_compatible, run_zkvm_execution},
    },
    fixture::{DEVNET_NAME, latest_devnet_fixtures},
};
use tracing::info;

/// CLI options for the devnet zkVM execution runner.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "zkvm_execution_devnet",
    about = "Run the latest devnet blocks through a release-backed stateless validator guest.",
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    /// zkVM to execute the guest on.
    #[arg(long)]
    zkvm: zkVMKind,
    /// Stateless validator guest to execute.
    #[arg(long)]
    stateless_validator: StatelessValidatorKind,
    /// Number of latest published block artifacts to run.
    #[arg(long, default_value_t = 100)]
    blocks: usize,
    /// Path to write the execution failures to.
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    init_tracing();

    if !is_guest_compatible(cli.stateless_validator, cli.zkvm) {
        info!(
            "Skipping {} on {}, the published ELF is not compatible with zkVM version {} of Ere",
            cli.stateless_validator,
            cli.zkvm,
            cli.zkvm.sdk_version()
        );
        return;
    }

    let fixtures = latest_devnet_fixtures(cli.blocks);
    info!(
        "Running {} {DEVNET_NAME} blocks from {} to {}",
        fixtures.len(),
        fixtures.first().unwrap().name,
        fixtures.last().unwrap().name,
    );
    let failures = run_zkvm_execution(cli.stateless_validator, cli.zkvm, fixtures);
    if let Some(output) = cli.output {
        fs::write(output, ExecutionFailures(&failures).to_string()).unwrap();
    }
}
