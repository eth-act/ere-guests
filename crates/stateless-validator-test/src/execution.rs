//! Test helpers for the stateless validator guests.

use std::{
    fmt::{self, Debug, Display},
    sync::Once,
    time::Instant,
};

use rayon::prelude::*;
use sha2::{Digest, Sha256};
use stateless_validator_common::{SszDecode, guest::StatelessValidationResult};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use crate::fixture::StatelessValidatorFixture;

pub mod host;
pub mod zkvm;

/// A stateless validator guest program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuestKind {
    /// The ethrex guest.
    Ethrex,
    /// The reth guest.
    Reth,
    /// The zesu guest.
    Zesu,
}

impl GuestKind {
    /// Returns the guest name in lower-case.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ethrex => "ethrex",
            Self::Reth => "reth",
            Self::Zesu => "zesu",
        }
    }
}

/// The public values written by a guest execution.
#[derive(Debug, Clone)]
pub enum ExecutionOutput {
    /// The stateless output bytes.
    Bytes(Vec<u8>),
    /// The sha256 digest of the stateless output bytes.
    Hash(Vec<u8>),
}

impl ExecutionOutput {
    fn matches(self, expected_stateless_output_bytes: Vec<u8>) -> anyhow::Result<()> {
        match self {
            Self::Bytes(stateless_output_bytes) => {
                let expected_stateless_output =
                    StatelessValidationResult::from_ssz_bytes(&expected_stateless_output_bytes)
                        .map_err(|err| {
                            anyhow::anyhow!("Decode fixture output bytes failure: {err:?}")
                        })?;
                if let Some((stateless_output_bytes, trailing)) =
                    stateless_output_bytes.split_at_checked(expected_stateless_output_bytes.len())
                    && trailing.iter().all(|byte| *byte == 0)
                {
                    match StatelessValidationResult::from_ssz_bytes(stateless_output_bytes) {
                        Ok(stateless_output) => {
                            if stateless_output != expected_stateless_output {
                                anyhow::bail!(
                                    "Output mismatch, expected {expected_stateless_output:?}, got {stateless_output:?}",
                                );
                            }
                        }
                        Err(err) => {
                            anyhow::bail!("Decode execute output bytes failure: {err:?}")
                        }
                    }
                } else {
                    anyhow::bail!(
                        "Output bytes mismatch, expected {}, got {}",
                        const_hex::encode_prefixed(expected_stateless_output_bytes),
                        const_hex::encode_prefixed(stateless_output_bytes)
                    )
                }
            }
            Self::Hash(stateless_output_hash) => {
                let expected_stateless_output_hash =
                    Sha256::digest(expected_stateless_output_bytes);

                if let Some((stateless_output_hash, trailing)) =
                    stateless_output_hash.split_at_checked(expected_stateless_output_hash.len())
                    && trailing.iter().all(|byte| *byte == 0)
                {
                    if stateless_output_hash != &expected_stateless_output_hash[..] {
                        anyhow::bail!(
                            "Output hash mismatch, expected {}, got {}",
                            const_hex::encode_prefixed(expected_stateless_output_hash),
                            const_hex::encode_prefixed(stateless_output_hash)
                        )
                    }
                } else {
                    anyhow::bail!(
                        "Output hash mismatch, expected {}, got {}",
                        const_hex::encode_prefixed(expected_stateless_output_hash),
                        const_hex::encode_prefixed(stateless_output_hash)
                    )
                }
            }
        };
        Ok(())
    }
}

/// A fixture that failed to execute or match its expected output.
#[derive(Debug, Clone)]
pub struct ExecutionFailure {
    /// Name of the failing fixture.
    name: String,
    /// Reason the fixture failed.
    err: String,
}

/// A [`Display`] view over a slice of [`ExecutionFailure`].
#[derive(Debug)]
pub struct ExecutionFailures<'a>(pub &'a [ExecutionFailure]);

impl Display for ExecutionFailures<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} execution failures:", self.0.len())?;
        for failure in self.0 {
            writeln!(f, "  - {}", failure.name)?;
            writeln!(f, "    {}", failure.err)?;
        }
        Ok(())
    }
}

/// Runs `execute` over every fixture in parallel, returning the failures.
pub fn run_execution(
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
    execute: &(impl Fn(Vec<u8>) -> anyhow::Result<ExecutionOutput> + Sync),
) -> Vec<ExecutionFailure> {
    static INIT_TRACING: Once = Once::new();
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    });

    let fixtures = fixtures.into_iter().collect::<Vec<_>>();
    let total = fixtures.len();
    assert!(total > 0);

    info!("Running execution of {total} fixtures...");

    let mut failures = fixtures
        .into_par_iter()
        .filter_map(|fixture| {
            let name = fixture.name;
            let start = Instant::now();
            if let Err(err) = execute(fixture.stateless_input_bytes)
                .and_then(|output| output.matches(fixture.stateless_output_bytes))
                .map_err(|err| err.to_string())
            {
                Some(ExecutionFailure { name, err })
            } else {
                debug!("PASS {name}: took {:?}", start.elapsed());
                None
            }
        })
        .collect::<Vec<_>>();
    failures.sort_by(|a, b| a.name.cmp(&b.name));

    info!("{} of {total} fixtures succeeded", total - failures.len());
    if !failures.is_empty() {
        info!("{}", ExecutionFailures(&failures));
    }

    failures
}
