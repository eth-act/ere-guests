//! Runs the latest blocks published in the devnet catalog through a stateless
//! validator guest program in a zkVM, reporting the failures on stdout.
//!
//! Run with env `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use the pre-built
//! image as the executor.
//!
//! Run with env `OPENVM_RUST_TOOLCHAIN=nightly-2026-01-18` for OpenVM to
//! compile Reth guest.

use clap::Parser;
use ere_dockerized::zkVMKind;
use serde::Deserialize;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{ExecutionFailures, zkvm::run_stateless_validator_execution},
    fixture::{R2_FIXTURES_BASE_URL, StatelessValidatorFixture, archive_fixtures},
};

/// Subdirectory under `<crate>/fixtures/` holding the unpacked devnet batches.
const DEVNET_FIXTURES_DIR: &str = "rpc-glamsterdam-devnet-7";

/// CLI options for the devnet zkVM execution runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Parser)]
#[command(
    name = "zkvm_execution_devnet",
    about = "Run the latest devnet blocks through a stateless validator guest in a zkVM.",
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
}

fn main() {
    let cli = Cli::parse();
    let fixtures = latest_devnet_fixtures(cli.blocks);
    let failures = run_stateless_validator_execution(cli.stateless_validator, cli.zkvm, fixtures);
    print!("{}", ExecutionFailures(&failures));
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
