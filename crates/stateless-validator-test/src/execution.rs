//! Test helpers for the stateless validator guests.

use std::{
    fmt::{self, Debug, Display},
    sync::Once,
    time::Instant,
};

use anyhow::{anyhow, bail};
use rayon::prelude::*;
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
    guest_kind: GuestKind,
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
    execute: &(impl Fn(Vec<u8>) -> anyhow::Result<Vec<u8>> + Sync),
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

    let matches_output = if matches!(guest_kind, GuestKind::Zesu) {
        matches_output_legacy
    } else {
        matches_output
    };

    let mut failures = fixtures
        .into_par_iter()
        .filter_map(|fixture| {
            let name = fixture.name;
            let start = Instant::now();
            if let Err(err) = execute(fixture.stateless_input_bytes)
                .and_then(|output| matches_output(output, fixture.stateless_output_bytes))
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

macro_rules! declare_matches_output {
    ($name:ident, $stateless_validator_common_crate:ident) => {
        fn $name(got_bytes: Vec<u8>, expectecd_bytes: Vec<u8>) -> anyhow::Result<()> {
            use $stateless_validator_common_crate::{SszDecode, guest::StatelessValidationResult};

            let Some(got_bytes) = got_bytes.split_at_checked(expectecd_bytes.len()).and_then(
                |(got_bytes, trailing)| trailing.iter().all(|byte| *byte == 0).then_some(got_bytes),
            ) else {
                bail!(
                    "Output bytes mismatch, expected {}, got {}",
                    const_hex::encode_prefixed(expectecd_bytes),
                    const_hex::encode_prefixed(got_bytes)
                )
            };

            let got = StatelessValidationResult::from_ssz_bytes(got_bytes)
                .map_err(|err| anyhow!("Decode execute output bytes failure: {err:?}"))?;
            let expected = StatelessValidationResult::from_ssz_bytes(&expectecd_bytes)
                .map_err(|err| anyhow!("Decode fixture output bytes failure: {err:?}"))?;

            match (
                expected.new_payload_request_root == got.new_payload_request_root,
                expected.successful_validation == got.successful_validation,
                expected.chain_config == got.chain_config,
            ) {
                (true, true, true) => Ok(()),
                (false, true, true) => bail!(
                    "Output new_payload_request_root mismatch, expected {}, got {}",
                    const_hex::encode_prefixed(expected.new_payload_request_root),
                    const_hex::encode_prefixed(got.new_payload_request_root)
                ),
                (true, false, true) => bail!(
                    "Output successful_validation mismatch, expected {}, got {}",
                    expected.successful_validation,
                    got.successful_validation
                ),
                (true, true, false) => bail!(
                    "Output chain_config mismatch, expected {:?}, got {:?}",
                    expected.chain_config,
                    got.chain_config
                ),
                _ => bail!("Output mismatch, expected {expected:?}, got {got:?}"),
            }
        }
    };
}

declare_matches_output!(matches_output, stateless_validator_common);
declare_matches_output!(matches_output_legacy, stateless_validator_common_legacy);
