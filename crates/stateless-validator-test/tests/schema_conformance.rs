//! `tests-zkevm@v21.0.7` schema conformance of `stateless-validator-common`.
//!
//! Decodes every EEST and Sepolia preset fixture input on the host and checks it against the
//! expected guest output. This needs no Docker or zkVM, so it checks the common crate against the
//! spec without any guest.

use rayon::prelude::*;
use stateless_validator_common::{
    HashTreeRoot, Sha2Hasher, SszDecode,
    guest::{StatelessInput, StatelessValidationResult},
};
use stateless_validator_test::{
    execution::init_tracing,
    fixture::{StatelessValidatorFixture, eest_fixtures, sepolia_preset_fixtures},
};

/// Checks that `fixture` decodes exactly when its expected output is not the all-zero sentinel,
/// and that the decoded input carries the expected root, chain ID and schema ID.
fn check_fixture(fixture: &StatelessValidatorFixture) -> Result<(), String> {
    let expected = StatelessValidationResult::from_ssz_bytes(&fixture.stateless_output_bytes)
        .map_err(|error| format!("failed to decode fixture output: {error:?}"))?;
    let (fork, input) =
        match StatelessInput::from_schema_prefixed_ssz(&fixture.stateless_input_bytes) {
            Ok(decoded) => decoded,
            Err(_) if expected == StatelessValidationResult::default() => return Ok(()),
            Err(error) => return Err(format!("decode failed with {error}, expected {expected:?}")),
        };
    let got = StatelessValidationResult {
        new_payload_request_root: input.new_payload_request.hash_tree_root(&Sha2Hasher),
        // The host cannot check the validation verdict without executing the payload.
        successful_validation: expected.successful_validation,
        chain_id: input.chain_id,
        schema_id: fork.schema_id(),
    };
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, decoded {got:?}"))
    }
}

/// Asserts that every fixture conforms, listing the ones that don't.
fn assert_conformance(fixtures: &[StatelessValidatorFixture]) {
    let mut failures = fixtures
        .par_iter()
        .filter_map(|fixture| {
            check_fixture(fixture)
                .err()
                .map(|error| format!("  - {}\n    {error}", fixture.name))
        })
        .collect::<Vec<_>>();
    failures.sort();
    assert!(
        failures.is_empty(),
        "{} of {} fixtures do not conform:\n{}",
        failures.len(),
        fixtures.len(),
        failures.join("\n"),
    );
}

#[test]
fn decodes_eest_inputs_to_expected_outputs() {
    init_tracing();
    let fixtures = eest_fixtures();
    assert!(
        fixtures
            .iter()
            .any(|fixture| fixture.stateless_input_bytes.is_empty()),
        "the empty-input rejection fixture must be loaded"
    );
    assert_conformance(&fixtures);
}

#[test]
fn decodes_sepolia_preset_inputs_to_expected_outputs() {
    init_tracing();
    assert_conformance(&sepolia_preset_fixtures());
}
