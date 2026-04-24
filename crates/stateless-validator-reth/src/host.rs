//! Implementations for host environment.

use alloc::{format, vec::Vec};
use std::sync::Arc;

use alloy_consensus::Transaction;
use alloy_eips::{Encodable2718, eip7685::Requests, eip7928::BlockAccessList};
use alloy_genesis::{ChainConfig, Genesis};
use alloy_primitives::U256;
use anyhow::Context;
use ere_prover_core::{Input, codec::Encode};
use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm_ethereum::EthEvmConfig;
use reth_primitives_traits::Block;
pub use stateless::StatelessInput;
use stateless::{UncompressedPublicKey, stateless_validation_with_trie};
pub use stateless_validator_common::guest::StatelessValidatorOutput;
use stateless_validator_common::new_payload_request::{
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, ExecutionPayloadV4, ExtraData,
    ForkName, NewPayloadRequest, Transaction as Tx, Transactions, Withdrawal, Withdrawals,
};
use tries::zeth::SparseState;

use crate::guest::StatelessValidatorRethInput;

impl StatelessValidatorRethInput {
    /// Construct [`StatelessValidatorRethInput`] given [`StatelessInput`].
    pub fn new(stateless_input: &StatelessInput, valid_block: bool) -> anyhow::Result<Self> {
        let signers = recover_signers(&stateless_input.block.body.transactions)?;
        let requests = get_requests(stateless_input, &signers, valid_block)?;
        let new_payload_request = to_new_payload_request(stateless_input, requests)?;

        Ok(Self {
            new_payload_request,
            witness: stateless_input.witness.clone(),
            chain_config: stateless_input.chain_config.clone(),
            public_keys: signers,
        })
    }

    /// Returns [`Input`] to [`zkVMProver`] methods.
    ///
    /// [`zkVMProver`]: ere_prover_core::zkVMProver
    pub fn to_zkvm_input(&self) -> anyhow::Result<Input> {
        Ok(Input::new().with_prefixed_stdin(self.encode_to_vec()?))
    }
}

/// Recover public keys from transaction signatures.
pub fn recover_signers<'a, I>(txs: I) -> anyhow::Result<Vec<UncompressedPublicKey>>
where
    I: IntoIterator<Item = &'a TransactionSigned>,
{
    txs.into_iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .map(|key| key.to_encoded_point(false).as_bytes().try_into().unwrap())
                .map(UncompressedPublicKey)
                .with_context(|| format!("failed to recover signature for tx #{i}"))
        })
        .collect()
}

/// Determines the fork name based on alloy chain config and block timestamp.
pub fn determine_fork_name(chain_config: &ChainConfig, timestamp: u64) -> ForkName {
    // Check forks in reverse chronological order
    if chain_config
        .amsterdam_time
        .is_some_and(|amsterdam_time| timestamp >= amsterdam_time)
    {
        return ForkName::Amsterdam;
    }
    if chain_config
        .osaka_time
        .is_some_and(|osaka_time| timestamp >= osaka_time)
    {
        return ForkName::Fulu;
    }
    if chain_config
        .prague_time
        .is_some_and(|prague_time| timestamp >= prague_time)
    {
        return ForkName::Electra;
    }
    if chain_config
        .cancun_time
        .is_some_and(|cancun_time| timestamp >= cancun_time)
    {
        return ForkName::Deneb;
    }
    if chain_config
        .shanghai_time
        .is_some_and(|shanghai_time| timestamp >= shanghai_time)
    {
        return ForkName::Capella;
    }
    ForkName::Bellatrix
}

/// Converts a [`StatelessInput`] to a [`NewPayloadRequest`].
fn to_new_payload_request(
    stateless_input: &StatelessInput,
    requests: Requests,
) -> anyhow::Result<NewPayloadRequest> {
    let header = stateless_input.block.header();
    let body = stateless_input.block.body();
    let fork = determine_fork_name(&stateless_input.chain_config, header.timestamp);

    // Convert transactions to RLP-encoded bytes
    let transactions: Transactions = {
        let txs: Vec<Tx> = body
            .transactions()
            .map(|tx| {
                let mut buf = Vec::new();
                tx.encode_2718(&mut buf);
                Tx::try_from(buf)
            })
            .collect::<Result<_, _>>()
            .map_err(|err| {
                anyhow::anyhow!("transaction length should be within bounds: {err:?}")
            })?;
        txs.try_into()
            .map_err(|err| anyhow::anyhow!("transactions count should be within bounds: {err:?}"))?
    };
    let logs_bloom = *header.logs_bloom.data();
    let extra_data: ExtraData = header
        .extra_data
        .to_vec()
        .try_into()
        .map_err(|err| anyhow::anyhow!("extra data length should be within bounds: {err:?}"))?;

    // Helper to convert alloy withdrawal to our neutral type
    let convert_withdrawal = |w: &alloy_eips::eip4895::Withdrawal| Withdrawal {
        index: w.index,
        validator_index: w.validator_index,
        address: w.address.0.0,
        amount: w.amount,
    };
    let build_withdrawals = || -> anyhow::Result<Withdrawals> {
        let wdls: Vec<Withdrawal> = body
            .withdrawals
            .as_ref()
            .map(|ws| ws.iter().map(convert_withdrawal).collect())
            .unwrap_or_default();
        wdls.try_into()
            .map_err(|err| anyhow::anyhow!("withdrawals length should be within bounds: {err:?}"))
    };

    match fork {
        ForkName::Bellatrix => {
            let payload = ExecutionPayloadV1 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom,
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: extra_data.clone(),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions: transactions.clone(),
            };
            Ok(NewPayloadRequest::new_bellatrix(payload))
        }
        ForkName::Capella => {
            let withdrawals = build_withdrawals()?;

            let payload = ExecutionPayloadV2 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom,
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: extra_data.clone(),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions: transactions.clone(),
                withdrawals,
            };
            Ok(NewPayloadRequest::new_capella(payload))
        }
        ForkName::Deneb => {
            let withdrawals = build_withdrawals()?;

            let payload = ExecutionPayloadV3 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom,
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: extra_data.clone(),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions: transactions.clone(),
                withdrawals,
                blob_gas_used: header.blob_gas_used.unwrap_or_default(),
                excess_blob_gas: header.excess_blob_gas.unwrap_or_default(),
            };

            // Collect blob versioned hashes from all blob transactions
            let versioned_hashes: Vec<[u8; 32]> = body
                .transactions()
                .filter_map(|tx| tx.blob_versioned_hashes())
                .flatten()
                .map(|h| h.0)
                .collect();

            let parent_beacon_block_root = stateless_input
                .block
                .parent_beacon_block_root
                .unwrap_or_default()
                .0;

            NewPayloadRequest::new_deneb(payload, versioned_hashes, parent_beacon_block_root)
        }
        ForkName::Electra | ForkName::Fulu => {
            let withdrawals = build_withdrawals()?;

            let payload = ExecutionPayloadV3 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom,
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data,
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions,
                withdrawals,
                blob_gas_used: header.blob_gas_used.unwrap_or_default(),
                excess_blob_gas: header.excess_blob_gas.unwrap_or_default(),
            };

            // Collect blob versioned hashes from all blob transactions
            let versioned_hashes: Vec<[u8; 32]> = body
                .transactions()
                .filter_map(|tx| tx.blob_versioned_hashes())
                .flatten()
                .map(|h| h.0)
                .collect();

            let parent_beacon_block_root = stateless_input
                .block
                .parent_beacon_block_root
                .unwrap_or_default()
                .0;

            NewPayloadRequest::new_electra_fulu(
                payload,
                versioned_hashes,
                parent_beacon_block_root,
                &requests,
            )
        }
        ForkName::Amsterdam => {
            let withdrawals = build_withdrawals()?;
            // TODO(Amsterdam): use actual BAL that should be received from parameter.
            let block_access_list = Tx::try_from(alloy_rlp::encode(BlockAccessList::default()))
                .map_err(|err| {
                    anyhow::anyhow!(
                        "default block access list length should be within bounds: {err:?}"
                    )
                })?;
            let slot_number = header.slot_number.unwrap_or_default();

            let payload = ExecutionPayloadV4 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom,
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data,
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions,
                withdrawals,
                blob_gas_used: header.blob_gas_used.unwrap_or_default(),
                excess_blob_gas: header.excess_blob_gas.unwrap_or_default(),
                block_access_list,
                slot_number,
            };

            let versioned_hashes: Vec<[u8; 32]> = body
                .transactions()
                .filter_map(|tx| tx.blob_versioned_hashes())
                .flatten()
                .map(|h| h.0)
                .collect();

            let parent_beacon_block_root = stateless_input
                .block
                .parent_beacon_block_root
                .unwrap_or_default()
                .0;

            NewPayloadRequest::new_amsterdam(
                payload,
                versioned_hashes,
                parent_beacon_block_root,
                &requests,
            )
        }
    }
}

fn get_requests(
    stateless_input: &StatelessInput,
    signers: &[UncompressedPublicKey],
    valid_block: bool,
) -> anyhow::Result<Requests> {
    if !valid_block {
        return Ok(Requests::default());
    }

    let genesis = Genesis {
        config: stateless_input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());
    let out = stateless_validation_with_trie::<SparseState, _, _>(
        stateless_input.block.clone(),
        signers.to_owned(),
        stateless_input.witness.clone(),
        chain_spec.clone(),
        evm_config,
    )
    .context("stateless validation failed")?;

    // This clone doesn't make much sense, but rust-analyzer can't figure out
    // why isn't required and mark it as error otherwise. Since this is only used
    // in the host side, we can afford the extra clone.
    Ok(out.execution_output.requests.clone())
}
