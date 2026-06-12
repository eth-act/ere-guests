//! Execution tests for `stateless-validator-ethrex` guest program

use ere_dockerized::zkVMKind;
use guest::Guest;
use stateless_validator_common::new_payload_request::ForkName;
use stateless_validator_ethrex::{
    guest::StatelessValidatorEthrexGuest,
    host::{Eip8025InputSource, build_eip8025_input},
};
use stateless_validator_reth::{
    guest::{StatelessValidatorRethGuest, StatelessValidatorRethInput},
    host::determine_fork_name,
};
use stateless_validator_test::{
    NoopPlatform, TestCase, get_fixtures, stateless_validator::get_stateless_validator_output,
};

fn test_execution(zkvm_kind: zkVMKind) {
    let fixtures = get_fixtures();
    let inputs = fixtures.into_iter().filter_map(|fixture| {
        let fork = determine_fork_name(
            &fixture.stateless_input.chain_config,
            fixture.stateless_input.block.header.timestamp,
        );
        if !matches!(fork, ForkName::Electra | ForkName::Fulu) {
            return None;
        }

        let output = if !fixture.success {
            let input = StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)
                .unwrap();
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
                fixture.stateless_input.chain_config.chain_id,
            )
        };
        assert_eq!(output.successful_block_validation, fixture.success);

        let input = build_eip8025_input(Eip8025InputSource::Legacy {
            stateless_input: &fixture.stateless_input,
            valid_block: fixture.success,
        })
        .unwrap();
        Some(
            TestCase::new::<StatelessValidatorEthrexGuest>(fixture.name, input, output)
                .output_sha256(),
        )
    });
    stateless_validator_test::test_execution("stateless-validator-ethrex", zkvm_kind, inputs);
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
