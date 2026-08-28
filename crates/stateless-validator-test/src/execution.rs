//! Test helpers for the stateless validator guests.

use std::{
    fmt::{self, Debug, Display},
    sync::Once,
    time::Instant,
};

use anyhow::bail;
use rayon::prelude::*;
use stateless_validator_common::{SszDecode, guest::StatelessValidationResult};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use crate::fixture::StatelessValidatorFixture;

pub mod zkvm;

const STATELESS_VALIDATION_RESULT_LEN: usize = 43;

/// A fixture that failed to execute or match its expected output.
#[derive(Debug, Clone)]
pub struct ExecutionFailure {
    /// Name of the failing fixture.
    pub name: String,
    /// Reason the fixture failed.
    pub error: String,
}

/// A [`Display`] view over a slice of [`ExecutionFailure`].
#[derive(Debug)]
pub struct ExecutionFailures<'a>(pub &'a [ExecutionFailure]);

impl Display for ExecutionFailures<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} execution failures:", self.0.len())?;
        for failure in self.0 {
            writeln!(f, "  - {}", failure.name)?;
            writeln!(f, "    {}", failure.error)?;
        }
        Ok(())
    }
}

/// Installs a global tracing subscriber.
pub fn init_tracing() {
    static INIT_TRACING: Once = Once::new();
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    });
}

/// Runs `execute` over every fixture in parallel, returning the failures.
pub fn run_execution(
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
    execute: &(impl Fn(Vec<u8>) -> anyhow::Result<Vec<u8>> + Sync),
) -> Vec<ExecutionFailure> {
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
                .and_then(|output| matches_output(output, fixture.stateless_output_bytes))
                .map_err(|err| err.to_string())
            {
                Some(ExecutionFailure { name, error: err })
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

/// Validates guest output against the canonical result, allowing zkVM word padding.
pub fn matches_output(got_bytes: Vec<u8>, expected_bytes: Vec<u8>) -> anyhow::Result<()> {
    let expected = StatelessValidationResult::from_ssz_bytes(&expected_bytes)
        .map_err(|error| anyhow::anyhow!("failed to decode fixture output: {error:?}"))?;

    let Some(got_bytes) = got_bytes
        .split_at_checked(STATELESS_VALIDATION_RESULT_LEN)
        .and_then(|(result, trailing)| trailing.iter().all(|byte| *byte == 0).then_some(result))
    else {
        bail!(
            "Output bytes mismatch, expected {}, got {}",
            const_hex::encode_prefixed(expected_bytes),
            const_hex::encode_prefixed(got_bytes)
        )
    };

    let got = StatelessValidationResult::from_ssz_bytes(got_bytes)
        .map_err(|error| anyhow::anyhow!("failed to decode guest output: {error:?}"))?;
    if got == expected {
        Ok(())
    } else {
        bail!("Output mismatch, expected {expected:?}, got {got:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::{STATELESS_VALIDATION_RESULT_LEN, matches_output};

    fn valid_output() -> Vec<u8> {
        let mut bytes = Vec::from([0xaa; STATELESS_VALIDATION_RESULT_LEN]);
        bytes[32] = 1;
        bytes[33..41].copy_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
        bytes[41..43].copy_from_slice(&0x1501_u16.to_le_bytes());
        bytes
    }

    #[test]
    fn accepts_fixed_v08_output() {
        let output = valid_output();
        matches_output(output.clone(), output).unwrap();
    }

    #[test]
    fn reports_malformed_fixture_and_guest_outputs_separately() {
        let mut malformed = valid_output();
        malformed[32] = 2;
        let fixture_error = matches_output(valid_output(), malformed.clone()).unwrap_err();
        assert!(
            fixture_error
                .to_string()
                .contains("failed to decode fixture output")
        );

        let guest_error = matches_output(malformed, valid_output()).unwrap_err();
        assert!(
            guest_error
                .to_string()
                .contains("failed to decode guest output")
        );

        for len in [0, STATELESS_VALIDATION_RESULT_LEN - 1] {
            assert!(matches_output(vec![0; len], valid_output()).is_err());
        }

        let mut long_fixture = valid_output();
        long_fixture.push(0);
        assert!(matches_output(valid_output(), long_fixture).is_err());
    }

    #[test]
    fn accepts_only_zero_word_padding() {
        let expected = valid_output();
        let mut padded = expected.clone();
        padded.extend([0; 5]);
        matches_output(padded, expected.clone()).unwrap();

        let mut nonzero_padding = expected.clone();
        nonzero_padding.push(1);
        assert!(matches_output(nonzero_padding, expected).is_err());
    }

    #[test]
    fn compares_decode_error_sentinel_distinctly() {
        let sentinel = vec![0; STATELESS_VALIDATION_RESULT_LEN];
        matches_output(sentinel.clone(), sentinel.clone()).unwrap();
        assert!(matches_output(sentinel, valid_output()).is_err());
    }
}
