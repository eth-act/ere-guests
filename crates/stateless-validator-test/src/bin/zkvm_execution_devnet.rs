//! Runs the latest blocks published in the devnet catalog through a stateless
//! validator guest program, in a zkVM or, when no zkVM is selected, natively on
//! the host, reporting the failures.
//!
//! Run with env `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use the pre-built
//! image as the executor.
//!
//! Run with env `OPENVM_RUST_TOOLCHAIN=nightly-2026-01-18` for OpenVM to
//! compile Reth guest.

use std::{fs, path::PathBuf};

use clap::Parser;
use ere_dockerized::zkVMKind;
use serde::Deserialize;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{
        ExecutionFailures, host::run_host_execution, init_tracing, zkvm::run_zkvm_execution,
    },
    fixture::{R2_FIXTURES_BASE_URL, StatelessValidatorFixture, archive_fixtures},
};
use tracing::info;

/// Subdirectory under `<crate>/fixtures/` holding the unpacked devnet batches.
const DEVNET_FIXTURES_DIR: &str = "rpc-glamsterdam-devnet-7";

/// CLI options for the devnet zkVM execution runner.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "zkvm_execution_devnet",
    about = "Run the latest devnet blocks through a stateless validator guest in a zkVM or on host.",
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    /// zkVM to execute the guest on, omit to execute the guest natively on host.
    #[arg(long)]
    zkvm: Option<zkVMKind>,
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
    let fixtures = latest_devnet_fixtures(cli.blocks);
    info!(
        "Running {} blocks from {} to {}",
        fixtures.len(),
        fixtures.first().unwrap().name,
        fixtures.last().unwrap().name,
    );
    let failures = match cli.zkvm {
        Some(zkvm) => run_zkvm_execution(cli.stateless_validator, zkvm, fixtures),
        None => run_host_execution(cli.stateless_validator, fixtures),
    };
    if let Some(output) = cli.output {
        fs::write(output, ExecutionFailures(&failures).to_string()).unwrap();
    }
}

/// Returns the fixtures of the latest `count` block artifacts published in the
/// devnet catalog, downloading and unpacking the batch archives covering them
/// into the local cache on first use.
fn latest_devnet_fixtures(count: usize) -> Vec<StatelessValidatorFixture> {
    let mut fixtures = latest_devnet_batches(count)
        .into_iter()
        .flat_map(|batch| {
            archive_fixtures(
                &format!(
                    "{DEVNET_FIXTURES_DIR}/{}-{}",
                    batch.batch_start_block, batch.batch_end_block
                ),
                &format!("{R2_FIXTURES_BASE_URL}/{}", batch.path),
                "blockchain_tests",
            )
        })
        .collect::<Vec<_>>();
    fixtures.drain(..fixtures.len().saturating_sub(count));
    fixtures
}

/// Returns the latest batches of the devnet catalog covering at least `count`
/// block artifacts, keeping the order of `batches.jsonl`.
fn latest_devnet_batches(count: usize) -> Vec<DevnetBatch> {
    let url = format!("{R2_FIXTURES_BASE_URL}/batches.jsonl");
    println!("Downloading devnet batch index {url}");
    let index = reqwest::blocking::get(&url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .text()
        .unwrap();

    let mut batches = index
        .lines()
        .map(|line| serde_json::from_str::<DevnetBatch>(line).unwrap())
        .collect::<Vec<_>>();
    let take = (batches
        .iter()
        .rev()
        .scan(0, |artifacts, batch| {
            *artifacts += batch.artifact_count;
            Some(*artifacts)
        })
        .take_while(|artifacts| *artifacts < count)
        .count()
        + 1)
    .min(batches.len());
    batches.split_off(batches.len() - take)
}

/// Wire shape of a batch entry in the devnet catalog's `batches.jsonl`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DevnetBatch {
    batch_start_block: u64,
    batch_end_block: u64,
    artifact_count: usize,
    path: String,
}
