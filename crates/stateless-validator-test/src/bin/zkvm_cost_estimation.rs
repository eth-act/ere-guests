//! Estimates the proving cost of one published guest ELF over a pinned slice of devnet blocks
//! and writes the per-block estimates as JSON.
//!
//! Run with env `ERE_IMAGE_REGISTRY=ghcr.io/eth-act/ere` to use the pre-built image as the
//! executor.

use std::{collections::BTreeMap, fs, num::NonZeroUsize, path::PathBuf, time::Instant};

use anyhow::Context;
use clap::Parser;
use ere_dockerized::{Elf, Input, zkVMKind};
use rayon::prelude::*;
use serde::Serialize;
use stateless_validator_catalog::StatelessValidatorKind;
use stateless_validator_test::{
    execution::{
        init_tracing, matches_output,
        zkvm::{download_artifact, init_zkvm, matches_up_to_patch},
    },
    fixture::{DEVNET_NAME, DevnetBatch, devnet_batches, devnet_fixtures, fetch_devnet_batches},
};
use tracing::{debug, info};

/// CLI options for the guest cost estimation runner.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "zkvm_cost_estimation",
    about = "Estimate the proving cost of one published guest ELF over pinned devnet blocks.",
    long_about = None,
    arg_required_else_help = true
)]
struct Cli {
    /// Stateless validator guest to execute.
    #[arg(long)]
    stateless_validator: StatelessValidatorKind,
    /// zkVM to execute the guest on.
    #[arg(long)]
    zkvm: zkVMKind,
    /// zkVM SDK version the ELF targets.
    #[arg(long)]
    zkvm_version: String,
    /// URL of the guest ELF.
    #[arg(long)]
    elf_url: String,
    /// SHA-256 of the guest ELF.
    #[arg(long)]
    elf_sha256: String,
    /// Last block of the newest devnet batch to run.
    #[arg(long)]
    batch_end_block: u64,
    /// Number of published block artifacts to run, ending with `batch_end_block`.
    #[arg(long, default_value = "100")]
    blocks: NonZeroUsize,
    /// Path to write the JSON report to.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Serialize)]
struct CostReport {
    stateless_validator: StatelessValidatorKind,
    zkvm: zkVMKind,
    zkvm_version: String,
    elf_sha256: String,
    fixture_set: &'static str,
    fixture_end_block: u64,
    blocks: Vec<BlockCostEstimation>,
}

#[derive(Debug, Serialize)]
struct BlockCostEstimation {
    name: String,
    #[serde(flatten)]
    outcome: BlockOutcome,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum BlockOutcome {
    Estimated {
        cost: BTreeMap<String, u64>,
        peak_heap_bytes: Option<u64>,
    },
    Failed {
        error: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing();

    if !matches_up_to_patch(&cli.zkvm_version, cli.zkvm.sdk_version()) {
        info!(
            "Skipping {} on {}, the ELF targets zkVM version {} but Ere provides {}",
            cli.stateless_validator,
            cli.zkvm,
            cli.zkvm_version,
            cli.zkvm.sdk_version()
        );
        return Ok(());
    }

    let zkvm = init_zkvm(
        cli.zkvm,
        Elf(download_artifact(&cli.elf_url, &cli.elf_sha256)),
    );
    let fixtures = {
        let batches = devnet_batches(&fetch_devnet_batches()?)?;
        let selected = selected_batches(&batches, cli.batch_end_block, cli.blocks)?;
        let mut fixtures = devnet_fixtures(selected);
        fixtures.drain(..fixtures.len().saturating_sub(cli.blocks.get()));
        fixtures
    };
    info!(
        "Estimating cost of {} {DEVNET_NAME} blocks from {} to {}",
        fixtures.len(),
        fixtures.first().unwrap().name,
        fixtures.last().unwrap().name,
    );

    let mut blocks = fixtures
        .into_par_iter()
        .map(|fixture| {
            let start = Instant::now();
            let outcome = zkvm
                .execute_estimated_cost(&Input::new().with_stdin(fixture.stateless_input_bytes))
                .and_then(|(public_values, estimation)| {
                    matches_output(public_values.to_vec(), fixture.stateless_output_bytes)?;
                    Ok(estimation)
                });
            let outcome = match outcome {
                Ok(estimation) => {
                    debug!("PASS {}: took {:?}", fixture.name, start.elapsed());
                    BlockOutcome::Estimated {
                        cost: estimation.cost,
                        peak_heap_bytes: estimation.peak_heap_bytes,
                    }
                }
                Err(error) => BlockOutcome::Failed {
                    error: error.to_string(),
                },
            };
            BlockCostEstimation {
                name: fixture.name,
                outcome,
            }
        })
        .collect::<Vec<_>>();
    blocks.sort_by(|a, b| a.name.cmp(&b.name));

    let estimated = blocks
        .iter()
        .filter(|block| matches!(block.outcome, BlockOutcome::Estimated { .. }))
        .count();
    info!("{estimated} of {} fixtures succeeded", blocks.len());

    let report = CostReport {
        stateless_validator: cli.stateless_validator,
        zkvm: cli.zkvm,
        zkvm_version: cli.zkvm_version,
        elf_sha256: cli.elf_sha256,
        fixture_set: DEVNET_NAME,
        fixture_end_block: cli.batch_end_block,
        blocks,
    };
    fs::write(cli.output, serde_json::to_string_pretty(&report)?)?;

    Ok(())
}

fn selected_batches(
    batches: &[DevnetBatch],
    end_block: u64,
    count: NonZeroUsize,
) -> anyhow::Result<&[DevnetBatch]> {
    let end = batches
        .iter()
        .position(|batch| batch.batch_end_block == end_block)
        .with_context(|| format!("no devnet batch ends at block {end_block}"))?
        + 1;
    let mut start = end;
    let mut blocks = 0;
    while start > 0 && blocks < count.get() {
        start -= 1;
        blocks += batches[start].artifact_count;
    }
    Ok(&batches[start..end])
}
