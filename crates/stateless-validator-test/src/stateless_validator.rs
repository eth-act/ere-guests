//! This module provides struct for stateless validator test fixture.

use std::collections::HashMap;

use alloy_primitives::{B256, b256};
use serde::{Deserialize, Serialize};
use stateless::StatelessInput;
use stateless_validator_common::guest::StatelessValidatorOutput;

/// A stateless validation fixture containing block data and witness information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorFixture {
    /// Name of the blockchain test case (e.g., "`ModExpAttackContract`").
    pub name: String,
    /// The stateless input for the block validation.
    pub stateless_input: StatelessInput,
    /// Whether the stateless block validation is successful.
    pub success: bool,
}

/// Returns the expected `StatelessValidatorOutput` for a given block hash in the fixtures, which
/// was calculated by an independent implementation.
pub fn get_stateless_validator_output(
    block_hash: B256,
    success: bool,
    chain_id: u64,
) -> StatelessValidatorOutput {
    let expected_roots = expected_execution_payload_tree_roots();
    let expected_root = *expected_roots.get(&block_hash).unwrap();

    StatelessValidatorOutput::new(expected_root.0, success, chain_id)
}

/// Returns a mapping of block hashes from fixtures to their expected execution payload tree roots.
/// These roots where independently computed from this repository.
fn expected_execution_payload_tree_roots() -> HashMap<B256, B256> {
    HashMap::from([
        (
            b256!("e4bd1c4dc22a58a0a9a8e789e2c54b4ace2d1ebc16a605c3976723b52fc011f1"),
            b256!("45328434f812b65daa21b4e8a3d6440d0da95fbd95a6c10b0a28f081cab53bd5"),
        ),
        (
            b256!("88c74fb93052f3bccc20e2ce709a97a6bd669cbdf1c3a54997b6f5cfda03accc"),
            b256!("f8ac723efd5d8d12af604948d0f46e53be47d18a23e6ac2adfbe3818ac1d0d98"),
        ),
        (
            b256!("77bd26b8aed0d14e1d78c180196de399840ee7462cb3be6f20981f63284b0bb7"),
            b256!("22de93d4f2bfcc6d35530d8ef3f57d60daec797a646a7ac66fbef1c240359f3d"),
        ),
        (
            b256!("4ea1ba2443afd99b07534559feb2d57c390489c2594c2a053e79ff066851db63"),
            b256!("2d4f61867176c82bab2c396559320b337b51412cfa8fbf988560b81f8ef4863c"),
        ),
        (
            b256!("e2154b8a9ad9fdc0e6e23dbd6be53b568cfe15da66d945272bd627ab488bc7fc"),
            b256!("66063b993a271908b668380e0e5ae2f915ccd993fd999bc75a7fc6e1cc5af9db"),
        ),
        (
            b256!("18e16e8807c5d723cfa8c1146474c63e0e334abedeb7bf4b1c3dfc44860d2db6"),
            b256!("760580ec7bb1dcc3b0a53fe0fec0f9dd94ab5c068b0e3c01128d11da7b5e679e"),
        ),
        (
            b256!("0853345a27fcf3de0d8f77be6465ccd638ab6fceff4f1aa95b2c8e3a20a94843"),
            b256!("3694ce5950dfbc84bf1ab6d3d54b52c269761e3e74420500a1c2e4a3e74efcda"),
        ),
        (
            b256!("f93cbd9836c96bc35a5f3e97fcd5887fa4cf4fead6d5b5fb796df34b76506d14"),
            b256!("ecb22240d38111f6236483b84cda9f96cff8a9b8c8ecb5cc3df89c396a030104"),
        ),
        (
            b256!("91ff4738a9ca92dc46ad86cfad7d6e31f678553d965357180537f4653bb48d9b"),
            b256!("c3c81e368dbbd449865caf4ef7721e19d6aa35b1b1480b58c4e412bf122f2558"),
        ),
        (
            b256!("2c576b118b1919886188c4a3b1f71143ca8b8a5c1814d8908beb004a6acb9fa1"),
            b256!("f490f0d4368609cf81fd90e04da0d9a1fe7cba9e3e21fc77e01968386affdc90"),
        ),
        (
            b256!("53dcdbca6be3f94cd96a8f37900082d04850bfc5a285e9a88b2498f318c89809"),
            b256!("359e5dee47560e9cffc8947ee8d75c2a5d15c621ec2c647426c95e4b69275c7f"),
        ),
    ])
}
