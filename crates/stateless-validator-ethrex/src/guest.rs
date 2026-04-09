//! Stateless validator guest program.

use alloc::{format, sync::Arc};
use core::fmt::Debug;

use ere_io::rkyv::IoRkyv;
use ethrex_common::types::block_execution_witness::ExecutionWitness;
use ethrex_crypto::Crypto;
use ethrex_guest_program::{execution::execution_program, input::ProgramInput};
use libssz_merkle::Sha256Hasher;
use stateless_validator_common::new_payload_request::NewPayloadRequest;

use crate::new_payload_request::get_block_from_new_payload_request;

#[rustfmt::skip]
pub use {
    guest::*,
    stateless_validator_common::guest::StatelessValidatorOutput,
};

/// Input for the Ethrex stateless validator guest program.
#[derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)]
pub struct StatelessValidatorEthrexInput {
    /// New payload request data.
    pub new_payload_request: NewPayloadRequest,
    /// database containing all the data necessary to execute
    pub execution_witness: ExecutionWitness,
}

impl Clone for StatelessValidatorEthrexInput {
    fn clone(&self) -> Self {
        Self {
            new_payload_request: self.new_payload_request.clone(),
            execution_witness: self.execution_witness.clone(),
        }
    }
}

impl Debug for StatelessValidatorEthrexInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct DebugExecutionWitness<'a>(&'a ExecutionWitness);

        impl Debug for DebugExecutionWitness<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct("ExecutionWitness")
                    .field("codes", &self.0.codes)
                    .field("block_headers_bytes", &self.0.block_headers_bytes)
                    .field("first_block_number", &self.0.first_block_number)
                    .field("chain_config", &self.0.chain_config)
                    .field("state_trie_root", &self.0.state_trie_root)
                    .field("storage_trie_roots", &self.0.storage_trie_roots)
                    .finish()
            }
        }

        f.debug_struct("StatelessValidatorEthrexInput")
            .field("new_payload_request", &self.new_payload_request)
            .field(
                "execution_witness",
                &DebugExecutionWitness(&self.execution_witness),
            )
            .finish()
    }
}

/// [`Io`] implementation of Ethrex stateless validator.
pub type StatelessValidatorEthrexIo =
    IoRkyv<StatelessValidatorEthrexInput, StatelessValidatorOutput>;

/// [`Guest`] implementation for Ethrex stateless validator.
#[derive(Debug, Clone)]
pub struct StatelessValidatorEthrexGuest;

struct EthrexSha256Hasher<'a> {
    crypto: &'a dyn Crypto,
}

impl<'a> EthrexSha256Hasher<'a> {
    fn new(crypto: &'a dyn Crypto) -> Self {
        Self { crypto }
    }
}

impl Sha256Hasher for EthrexSha256Hasher<'_> {
    fn hash(&self, data: &[u8]) -> [u8; 32] {
        self.crypto.sha256(data)
    }
}

impl Guest for StatelessValidatorEthrexGuest {
    type Io = StatelessValidatorEthrexIo;

    fn compute<P: Platform>(input: GuestInput<Self>) -> GuestOutput<Self> {
        let crypto = crypto();
        let new_payload_request_root =
            P::cycle_scope("new_payload_request_root_calculation", || {
                let hasher = EthrexSha256Hasher::new(crypto.as_ref());
                input.new_payload_request.tree_hash_root(&hasher)
            });

        #[cfg(feature = "std")]
        {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Self::compute_inner::<P>(input, new_payload_request_root, crypto.clone())
            }));

            match result {
                Ok(output) => output,
                Err(_) => {
                    P::print("Panic occurred during validation\n");
                    StatelessValidatorOutput::new(new_payload_request_root, false)
                }
            }
        }

        #[cfg(not(feature = "std"))]
        {
            Self::compute_inner::<P>(input, new_payload_request_root, crypto)
        }
    }
}

impl StatelessValidatorEthrexGuest {
    fn compute_inner<P: Platform>(
        input: GuestInput<Self>,
        new_payload_request_root: [u8; 32],
        crypto: Arc<dyn Crypto>,
    ) -> GuestOutput<Self> {
        let block_res = P::cycle_scope("new_payload_request_to_block", || {
            let hasher = EthrexSha256Hasher::new(crypto.as_ref());
            get_block_from_new_payload_request(input.new_payload_request, &hasher, crypto.as_ref())
        });
        let block = match block_res {
            Ok(block) => block,
            Err(err) => {
                P::print(&format!("Block construction failed: {err}\n"));
                return StatelessValidatorOutput::new(new_payload_request_root, false);
            }
        };

        let (input, block_num) = P::cycle_scope("misc_preparation", || {
            let input = ProgramInput {
                blocks: vec![block],
                execution_witness: input.execution_witness,
            };
            let block_num = input.blocks[0].header.number;
            (input, block_num)
        });

        let res = P::cycle_scope("stf", || execution_program(input, crypto));

        match res {
            Ok(_) => StatelessValidatorOutput::new(new_payload_request_root, true),
            Err(err) => {
                P::print(&format!("Block {} validation failed: {err}\n", block_num));
                StatelessValidatorOutput::new(new_payload_request_root, false)
            }
        }
    }
}

#[allow(unreachable_code)]
fn crypto() -> Arc<dyn Crypto> {
    #[cfg(feature = "risc0")]
    return Arc::new(ethrex_guest_program::crypto::risc0::Risc0Crypto);
    #[cfg(feature = "sp1")]
    return Arc::new(ethrex_guest_program::crypto::sp1::Sp1Crypto);
    #[cfg(feature = "zisk")]
    return Arc::new(ethrex_guest_program::crypto::zisk::ZiskCrypto);
    #[cfg(not(any(feature = "risc0", feature = "sp1", feature = "zisk")))]
    return Arc::new(ethrex_guest_program::crypto::NativeCrypto);
}

#[cfg(test)]
mod test {
    use stateless_validator_common::new_payload_request::{
        ExecutionPayloadV1, NativeSha256Hasher, NewPayloadRequest, NewPayloadRequestBellatrix,
    };

    use crate::guest::{Io, StatelessValidatorEthrexIo, StatelessValidatorOutput};

    #[test]
    fn serialize_output() {
        let dummy_new_payload_request_root =
            NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix {
                execution_payload: ExecutionPayloadV1 {
                    parent_hash: [1; 32],
                    fee_recipient: [2; 20],
                    state_root: [3; 32],
                    receipts_root: [4; 32],
                    logs_bloom: [0; 256],
                    prev_randao: [5; 32],
                    block_number: 1,
                    gas_limit: 2,
                    gas_used: 3,
                    timestamp: 4,
                    extra_data: Default::default(),
                    base_fee_per_gas: [6; 32],
                    block_hash: [7; 32],
                    transactions: Default::default(),
                },
            })
            .tree_hash_root(&NativeSha256Hasher);

        for output in [
            StatelessValidatorOutput::new(dummy_new_payload_request_root, false),
            StatelessValidatorOutput::new(dummy_new_payload_request_root, true),
        ] {
            assert_eq!(
                StatelessValidatorEthrexIo::serialize_output(&output).unwrap(),
                output.serialize()
            );
        }
    }
}
