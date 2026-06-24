//! Ethrex stateless validator guest program.

use alloc::vec::Vec;

use ere_platform_core::Platform;
use ethrex_guest_program::{
    common::ExecutionError,
    l1::{DecodedEip8025, validate_eip8025_canonical_execution, validate_eip8025_execution},
};
use stateless_validator_common::{
    HashTreeRoot, SszEncode as _,
    guest::{StatelessInput, StatelessValidationResult},
};

use crate::guest::{
    convert::to_ethrex_input,
    crypto::{sha256, sha256_hasher},
};

mod convert;
mod crypto;
mod error;

pub use crate::guest::error::Error;

/// Runs the stateless guest on the [`Platform`].
///
/// The public values written diverge from the spec by hashing the output with
/// sha256 because some zkVMs only support 32 byte public values.
pub fn entrypoint<P: Platform>() {
    let input_bytes = P::cycle_scope("read_input", || P::read_input());
    let output_bytes = run_stateless_guest::<P>(&input_bytes);
    let output_digest = P::cycle_scope("sha256_output_bytes", || sha256(&output_bytes));
    P::cycle_scope("write_output", || P::write_output(&output_digest));
}

/// Runs the stateless guest with serialized input and returns serialized
/// output, mirroring `run_stateless_guest` in the spec.
pub fn run_stateless_guest<P: Platform>(input_bytes: &[u8]) -> Vec<u8> {
    let Ok(input) = P::cycle_scope("deserialize_input", || {
        StatelessInput::from_schema_prefixed_ssz(input_bytes)
    }) else {
        return StatelessValidationResult::default().to_ssz();
    };

    let new_payload_request_root = P::cycle_scope("new_payload_request_root", || {
        input.new_payload_request.hash_tree_root(&sha256_hasher())
    });
    let chain_config = input.chain_config.clone();

    let successful_validation = verify_stateless_new_payload::<P>(input).is_ok();

    let output = StatelessValidationResult::new(
        new_payload_request_root,
        successful_validation,
        chain_config,
    );

    P::cycle_scope("serialize_output", || output.to_ssz())
}

/// Statelessly validates the execution payload, mirroring
/// `verify_stateless_new_payload` in the spec.
fn verify_stateless_new_payload<P: Platform>(input: StatelessInput) -> Result<(), Error> {
    P::cycle_scope("validate_chain_config", || {
        input.chain_config.validate(&input.new_payload_request)
    })?;

    let ethrex_input = P::cycle_scope("to_ethrex_input", || {
        to_ethrex_input(input).map_err(|err| {
            P::print(&format!("Input conversion failed: {err}\n"));
            err
        })
    })?;

    P::cycle_scope("run_validation", || {
        run_validation(ethrex_input).map_err(|err| {
            P::print(&format!("Validation failed: {err}\n"));
            err
        })
    })?;

    Ok(())
}

/// Validates the decoded payload through its canonical or legacy execution
/// path, reporting a rejected payload as an error.
fn run_validation(ethrex_input: DecodedEip8025) -> Result<(), ExecutionError> {
    match ethrex_input {
        DecodedEip8025::Legacy {
            new_payload_request,
            execution_witness,
        } => validate_eip8025_execution(&new_payload_request, execution_witness, crypto::crypto()),
        DecodedEip8025::Canonical {
            stateless_input,
            chain_config,
        } => validate_eip8025_canonical_execution(stateless_input, chain_config, crypto::crypto()),
    }
}
