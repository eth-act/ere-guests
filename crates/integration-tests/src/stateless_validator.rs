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
pub fn get_stateless_validator_output(block_hash: B256, success: bool) -> StatelessValidatorOutput {
    let expected_roots = expected_execution_payload_tree_roots();
    let expected_root = *expected_roots.get(&block_hash).unwrap();

    StatelessValidatorOutput::new(expected_root.0, success)
}

/// Returns a mapping of block hashes from fixtures to their expected execution payload tree roots.
/// These roots where independently computed from this repository.
fn expected_execution_payload_tree_roots() -> HashMap<B256, B256> {
    HashMap::from([
        (
            b256!("e6e4c256069674f7939f82fc808d0cd104210533c83add12d2c33d274fc3c027"),
            b256!("043b1e44af00a6ff2b3c0570404d8b6701fe6221ed6aa3a39856c835f1ccdec4"),
        ),
        (
            b256!("74356579507633dcd34faa38c64f8ec46bc23ab5c13bbb1f2ce46786147baf54"),
            b256!("03a7ab28fe855a1ba21024171c3b235e7a2e0206d2f0deefa50e186daf02fcf9"),
        ),
        (
            b256!("f72d095aaf5db3e99dbb76ec7f1dee9e6a3fe4cda536c073c7403ff160be356c"),
            b256!("9acbdd5767796bc24c3afbfa52f56fec54636d0e78db03032abdfa8e6f81c9dd"),
        ),
        (
            b256!("eca0cdd3433d05468326534f1fd7b64a23b7d01c3cec0791f4c5e16e0caa4228"),
            b256!("693e63f3b1db57e43f00161ef24ce591980dfc97cc0fa15fd6c1502f4dc97cec"),
        ),
        (
            b256!("bdee559b347d195bd65a82cac27533e2b9f94a5ba9dfb662e05033d12fb0ca4d"),
            b256!("ae730c3c051710c9b18e48c7a6cbf011aa967e93d32cd969e548030c8fc1216a"),
        ),
        (
            b256!("8466849d8c0855c92e9732b70f58e0d228a03d7741f7f0344ad9457eda4dab99"),
            b256!("3d1e280f1df9e380aefee8d6ff2cf10cd508e29812957ea53be7585b995781a0"),
        ),
        (
            b256!("b19d0861de72dbc6f40d6a118b05a975c3b7a525ad19db37b2bd975d60f0648f"),
            b256!("fe081fe4f17e96ff956517e23e204245cce07a68e18a40489f19fc2223403d48"),
        ),
        (
            b256!("7c6cf5941884a4c9a3183bf4d8c0025e771838929ea3651353f9a09ddb0f56de"),
            b256!("0000000000000000000000000000000000000000000000000000000000000000"),
        ),
        (
            b256!("2c041b9467dcf0899f85681d164d7edb08b992a552e796761c50d88dbffb6598"),
            b256!("8d129e566ca042f959abf3e1bc5d0e21c2a6a52d381f1d906e0a8665aeff14fe"),
        ),
        (
            b256!("c8e14160123e5e2f8037857b7fdc414bc1687b7c9173218c7ba25320e9448f24"),
            b256!("befe3b02a08353c0c07511f009590d01503ad076d34db02447ded858b02389d6"),
        ),
        (
            b256!("a6ee6b71a5c245a00e2724f3e92cf1b25e12fcc8844a343c241e00020d48500e"),
            b256!("957785c5a0d6008eb28e2d01a8a19c708cb07e69a3d8ac3c5e7c0b8557e046ab"),
        ),
        (
            b256!("cdaf26ec02a13a84ca0a3fc0047584290e57eb972dff3d19ebf2978733f1735f"),
            b256!("89134081d1d828eeba109aa97ff5714af6d9d2f44efaa48da62e81916f49c9ad"),
        ),
        (
            b256!("c8ac491bec27d1fbf6fa9e894b4f1ba593491e84bb593b9a81dfb89f29027149"),
            b256!("5743a40b8ffd635ec3e50c4f1833fd6734af834ac43189c4bf50c9df3b18c2b9"),
        ),
        (
            b256!("e4bd1c4dc22a58a0a9a8e789e2c54b4ace2d1ebc16a605c3976723b52fc011f1"),
            b256!("45328434f812b65daa21b4e8a3d6440d0da95fbd95a6c10b0a28f081cab53bd5"),
        ),
        (
            b256!("ba11cc5f2a0d42cc2d1c6ecee10b0c2c3c17dc685b17584be3474d6cafb14140"),
            b256!("4e5881a6977b69b39787accd970dafb9138bc8580046be01b7b073a91cd3eaec"),
        ),
        (
            b256!("444460fa6bf40df3a2b419d55450fb68424c3b5dff248581afb87741be7f92b9"),
            b256!("0cb48a874df4018608d53f09a025cad2977ffb06881cc6d2d07382c8280a2ce3"),
        ),
        (
            b256!("cec65cbf796165f17dc68b583aff9bb8e2f5ccd0fb41c03ac53d57b4740b6534"),
            b256!("895c7d51f58aeab5ec891651de307043efd42300338db3dbaf0d1e36dc13138c"),
        ),
    ])
}
