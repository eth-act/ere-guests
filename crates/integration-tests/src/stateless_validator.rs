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

/// Returns the pinned `StatelessValidatorOutput` for a given block hash in the
/// integration fixtures.
pub fn get_stateless_validator_output(block_hash: B256, success: bool) -> StatelessValidatorOutput {
    let expected_roots = expected_execution_payload_tree_roots();
    let expected_root = *expected_roots.get(&block_hash).unwrap();

    StatelessValidatorOutput::new(expected_root.0, success)
}

/// Returns the expected execution payload tree roots for the current
/// integration fixtures.
///
/// These values were regenerated with:
/// `cargo run -p stateless-validator-debug -- --guest reth --format rust-map <fixtures-dir>`
fn expected_execution_payload_tree_roots() -> HashMap<B256, B256> {
    HashMap::from([
        (
            b256!("6511c1eaebf501515e25d88dedbc3995812429e86c7b5a92d73b415be355d5de"),
            b256!("d46f3343ae4efb361730acd43ecf05130bfbdae44c962072dc6bd2eca8cf1c85"),
        ), // rpc_block_26606
        (
            b256!("8eeb5f69aff864829e77f7ab28d8b2dc4705e1943b0fe9c017e4b26eef846d06"),
            b256!("905d5374b850e0e2f9bb63beacf25f6592a9d5850fd7ce412b9a7e0aa2e77b87"),
        ), // rpc_block_26607
        (
            b256!("cdce57a2e72a0a10729775add24cb02793679767209236cb1d0bdd0ce8328532"),
            b256!("cc2aee441742dd49d237cd32b9567f282dc4e7419a308f57452b34a1e23cc676"),
        ), // rpc_block_26608
        (
            b256!("8b1e41dc49d744b6fac589a7200fa3ce88d30719a7699d01fa84261fdfb1bbce"),
            b256!("60f2a768db256eaed7cc361840b1ac4e3ba93338c0e6a50a3383c92caec153f2"),
        ), // rpc_block_26609
        (
            b256!("c43e1bc7137b62de7261c8c66556c49ecc2da720adfb6e02e153e411abeddddd"),
            b256!("c4790abad35adccb1adb64189bd1258a81dbb4cbe09dcecb51f0f46c9059ff39"),
        ), // rpc_block_26610
        (
            b256!("88eac3c594d529be51aded5c12d4283b694db6d3c11af02b341614ef8893e2c0"),
            b256!("c2d913c73e91d93fe7b6f596cb30e3de58463556894985a6a7f30143319f051d"),
        ), // rpc_block_26611
        (
            b256!("5b40eeff4a7ca37cfbca2f8e116c7516a2242ea55c17c3017df622826fd4d44a"),
            b256!("d64cb9ebf4ff6154e8426582c9fba4037cc90928d07acc5f597b332a04577d1a"),
        ), // rpc_block_26612
        (
            b256!("a2907992d2b983bb40f34e63e16d920bc8e0d87f05d3437e060ec237362f8588"),
            b256!("15a9925e61f0e5f51402eddd12dfbafa887e4b7c6e691095da281c080ba90b34"),
        ), // rpc_block_26613
        (
            b256!("3f1aa8ddf8cceaecda48600a58777f3bb370b269aa61084a1283072b5784a96d"),
            b256!("c5bf1c1a14c689fc2f4cdd528399d6a04e9cba0c82565d52ed33850f7ad972c6"),
        ), // rpc_block_26614
        (
            b256!("2affaae9152951e1652d7d1bbe19dca045ef548b60f18d4f70b26b8076acdb1a"),
            b256!("15590f765930927df902a93a98842a6277eb767321e1f73f047a56f202f2a62b"),
        ), // rpc_block_26615
        (
            b256!("42a82a2afe91f2db524114f77de54f03501c6828684c0633385b2d08e79f71d7"),
            b256!("0fe87a8bdf9a596acab0e43e0b6c778ebe8a87b6da36d30884c076ddcdaa1337"),
        ), // rpc_block_26616
        (
            b256!("21c57349ddf1b818ccba3b95c0f27b7aa3a31f8b0a34a3f1b4f52619584a60a5"),
            b256!("e90b20d6f304b767741ca008d18aa7678c7ca1ac65b7649ed8e4478c9ee3e0b5"),
        ), // rpc_block_26617
        (
            b256!("f40779a3fd33f7246d3eebdf040288567910c817b556425540344811cd0798f0"),
            b256!("884a4f610e764a126425175e6f658393ca70b94d6f8897e48efc0da4f2fc8b21"),
        ), // rpc_block_26618
        (
            b256!("32b60d182c2c475751bd9e34dc7eab6bdd545c29f0314478e672612cda942b94"),
            b256!("67c42c3374d834f6f7cc6a6ee47c0a129d124e7bdf9faaaef597ecb441682d33"),
        ), // rpc_block_26619
        (
            b256!("3ec2c72654168b1ef53b5f20bd6c9f5ba7d5f66a87182f49e634513eef9079d1"),
            b256!("9d3f2015af7b61b5e93a247acfad00608106c91be362e2255f703a8b0a346cab"),
        ), // rpc_block_26620
    ])
}
