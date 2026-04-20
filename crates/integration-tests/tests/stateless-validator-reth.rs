//! Execution tests for `stateless-validator-reth` guest program

use ere_dockerized::zkVMKind;
use guest::Guest;
use integration_tests::{
    NoopPlatform, TestCase, get_fixtures, stateless_validator::get_stateless_validator_output,
};
use stateless_validator_reth::guest::{StatelessValidatorRethGuest, StatelessValidatorRethInput};

fn test_execution(zkvm_kind: zkVMKind) {
    let fixtures = get_fixtures();
    let inputs = fixtures.into_iter().map(|fixture| {
        let input =
            StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success).unwrap();

        let output = if !fixture.success {
            // For invalid blocks we can't correctly generate the NewPayloadRequest
            // from an EL block. This is because to get the Electra requests, we
            // need to execute the block successfully first.
            StatelessValidatorRethGuest::compute::<NoopPlatform>(input.clone())
        } else {
            // For valid blocks (i.e. mainnet), we can rely on testing the output against an
            // independent implementation that calculated the NewPayloadRequest root
            // from a CL block.
            get_stateless_validator_output(
                fixture.stateless_input.block.hash_slow(),
                fixture.success,
            )
        };
        assert_eq!(output.successful_block_validation, fixture.success);

        TestCase::new::<StatelessValidatorRethGuest>(fixture.name, input, output).output_sha256()
    });
    integration_tests::test_execution("stateless-validator-reth", zkvm_kind, inputs);
}

#[test]
fn test_execution_airbender() {
    test_execution(zkVMKind::Airbender);
}

#[test]
fn test_execution_openvm() {
    test_execution(zkVMKind::OpenVM);
}

#[test]
fn test_execution_risc0() {
    test_execution(zkVMKind::Risc0);
}

#[test]
fn test_execution_sp1() {
    test_execution(zkVMKind::SP1);
}

#[test]
fn test_execution_zisk() {
    test_execution(zkVMKind::Zisk);
}
