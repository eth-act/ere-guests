//! Test for StatelessInput <-> NewPayloadRequest <-> EL block conversions
use std::sync::Arc;

use stateless::Genesis;
use stateless_validator_reth::{
    guest::StatelessValidatorRethInput, new_payload_request::new_payload_request_to_block,
};
use stateless_validator_test::get_fixtures;

#[test]
fn test_new_payload_request_el_block_roundtrip() {
    for fixture in get_fixtures() {
        // Simulate the preparation the prover does to send input to the guest.
        let input =
            StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success).unwrap();
        let new_payload_request = input.new_payload_request;

        // Do the work that the guest program does to reconstruct the block.
        let genesis = Genesis {
            config: fixture.stateless_input.chain_config.clone(),
            ..Default::default()
        };
        let chain_spec = Arc::new(genesis.into());
        let guest_block = new_payload_request_to_block(new_payload_request, chain_spec).unwrap();

        // Get the block hash computed by the guest.
        let guest_block_hash = guest_block.hash_slow();

        // Original block hash from StatelessInput.
        let stateless_input_block_hash = fixture.stateless_input.block.hash_slow();

        // Assert that the EL_Block -> NewPayloadRequest -> EL_Block roundtrip is consistent with
        // the original block in StatelessInput.
        assert_eq!(
            stateless_input_block_hash, guest_block_hash,
            "Block hash mismatch for fixture: {}",
            fixture.name
        );
    }
}
