//! Legacy input bridge.
//!
//! Converts the [`reth_stateless::StatelessInput`] format into the canonical [`StatelessInput`].
//! This module is the only place where execution layer dependencies participate in input
//! construction.

use std::sync::Arc;

use alloy_consensus::Transaction as _;
use alloy_eips::{Encodable2718, eip7685::Requests};
use alloy_genesis::Genesis;
use alloy_primitives::U256;
use anyhow::{Context, Result};
use libssz::{SszDecode, SszEncode};
use libssz_types::SszList;
use reth_chainspec::ChainSpec;
use reth_ethereum_primitives::TransactionSigned;
use reth_evm_ethereum::EthEvmConfig;
use reth_primitives_traits::Block;
use reth_stateless::{UncompressedPublicKey, stateless_validation_with_trie};
use reth_tries::zeth::SparseState;

use crate::guest::input::{
    BlobSchedule, ChainConfig, ExecutionWitness, ForkActivation, ForkConfig, ProtocolFork,
    StatelessInput,
    new_payload_request::{
        ConsolidationRequest, DepositRequest, ExecutionPayloadV1, ExecutionPayloadV2,
        ExecutionPayloadV3, ExecutionPayloadV4, ExecutionRequests, NewPayloadRequest,
        NewPayloadRequestBellatrix, NewPayloadRequestCapella, NewPayloadRequestDeneb,
        NewPayloadRequestElectraFulu, NewPayloadRequestGloas, Withdrawal, WithdrawalRequest,
    },
};

impl StatelessInput {
    /// Converts a legacy [`reth_stateless::StatelessInput`] into the canonical input.
    ///
    /// Configurations that merged through terminal total difficulty carry no
    /// merge block, so Paris resolves as active since genesis and blocks
    /// before the merge are not supported.
    pub fn try_from_reth(stateless_input: &reth_stateless::StatelessInput) -> Result<Self> {
        let signers = recover_signers(&stateless_input.block.body.transactions)?;
        let requests = compute_requests(stateless_input, &signers);

        let header = stateless_input.block.header();
        let chain_config = to_canonical_chain_config(
            &stateless_input.chain_config,
            header.number,
            header.timestamp,
        )?;
        let new_payload_request =
            to_new_payload_request(stateless_input, requests, chain_config.active_fork.fork)?;
        let witness = to_canonical_witness(&stateless_input.witness)?;
        let public_keys = ssz_list(signers.iter().map(|key| key.0).collect(), "public keys")?;

        Ok(Self {
            new_payload_request,
            witness,
            chain_config,
            public_keys,
        })
    }
}

/// Recovers public keys from transaction signatures.
fn recover_signers<'a, I>(txs: I) -> Result<Vec<UncompressedPublicKey>>
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

/// Reconstructs the EIP-7685 requests through a host-side validation run.
/// Validation failure yields empty requests, since an invalid block fails the
/// guest validation regardless of the requests value.
fn compute_requests(
    stateless_input: &reth_stateless::StatelessInput,
    signers: &[UncompressedPublicKey],
) -> Requests {
    let genesis = Genesis {
        config: stateless_input.chain_config.clone(),
        ..Default::default()
    };
    let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
    let evm_config = EthEvmConfig::new(chain_spec.clone());
    stateless_validation_with_trie::<SparseState, _, _>(
        stateless_input.block.clone(),
        signers.to_owned(),
        stateless_input.witness.clone(),
        chain_spec.clone(),
        evm_config,
    )
    .map(|out| out.execution_output.result.requests)
    .unwrap_or_default()
}

/// Reduces a full [`alloy_genesis::ChainConfig`] to the canonical chain
/// configuration holding only the fork active at the given block.
fn to_canonical_chain_config(
    chain_config: &alloy_genesis::ChainConfig,
    block_number: u64,
    timestamp: u64,
) -> Result<ChainConfig> {
    Ok(ChainConfig {
        chain_id: chain_config.chain_id,
        active_fork: active_fork(chain_config, block_number, timestamp)?,
    })
}

/// Resolves the fork active at the given block along with its activation
/// point and blob schedule.
fn active_fork(
    config: &alloy_genesis::ChainConfig,
    block_number: u64,
    timestamp: u64,
) -> Result<ForkConfig> {
    // The canonical protocol fork enumeration has no BPO3 to BPO5 entries,
    // so configurations scheduling them are not supported.
    for (time, name) in [
        (config.bpo3_time, "bpo3"),
        (config.bpo4_time, "bpo4"),
        (config.bpo5_time, "bpo5"),
    ] {
        anyhow::ensure!(time.is_none(), "scheduled fork {name} is not supported");
    }

    let by_time = |at: Option<u64>| -> (Option<u64>, Option<u64>) { (None, at) };
    let by_block = |at: Option<u64>| -> (Option<u64>, Option<u64>) { (at, None) };
    let forks = [
        (by_time(config.amsterdam_time), ProtocolFork::Amsterdam),
        (by_time(config.bpo2_time), ProtocolFork::BPO2),
        (by_time(config.bpo1_time), ProtocolFork::BPO1),
        (by_time(config.osaka_time), ProtocolFork::Osaka),
        (by_time(config.prague_time), ProtocolFork::Prague),
        (by_time(config.cancun_time), ProtocolFork::Cancun),
        (by_time(config.shanghai_time), ProtocolFork::Shanghai),
        (
            by_block(
                config
                    .merge_netsplit_block
                    .or(config.terminal_total_difficulty_passed.then_some(0)),
            ),
            ProtocolFork::Paris,
        ),
        (
            by_block(config.gray_glacier_block),
            ProtocolFork::GrayGlacier,
        ),
        (
            by_block(config.arrow_glacier_block),
            ProtocolFork::ArrowGlacier,
        ),
        (by_block(config.london_block), ProtocolFork::London),
        (by_block(config.berlin_block), ProtocolFork::Berlin),
        (
            by_block(config.muir_glacier_block),
            ProtocolFork::MuirGlacier,
        ),
        (by_block(config.istanbul_block), ProtocolFork::Istanbul),
        (
            by_block(config.petersburg_block),
            ProtocolFork::ConstantinopleFix,
        ),
        (
            by_block(config.constantinople_block),
            ProtocolFork::Constantinople,
        ),
        (by_block(config.byzantium_block), ProtocolFork::Byzantium),
        (
            by_block(config.eip158_block.or(config.eip155_block)),
            ProtocolFork::SpuriousDragon,
        ),
        (
            by_block(config.eip150_block),
            ProtocolFork::TangerineWhistle,
        ),
        (by_block(config.dao_fork_block), ProtocolFork::DAOFork),
        (by_block(config.homestead_block), ProtocolFork::Homestead),
    ];
    let ((at_block, at_time), fork) = forks
        .into_iter()
        .find(|((at_block, at_time), _)| {
            at_block.is_some_and(|at| block_number >= at)
                || at_time.is_some_and(|at| timestamp >= at)
        })
        .unwrap_or(((Some(0), None), ProtocolFork::Frontier));

    let blob_schedule = if let Some(schedule_key) = schedule_key(fork) {
        let params = config
            .blob_schedule
            .get(schedule_key)
            .with_context(|| format!("missing blob schedule entry for {schedule_key}"))?;
        Some(BlobSchedule {
            target: params.target_blob_count,
            max: params.max_blob_count,
            base_fee_update_fraction: u64::try_from(params.update_fraction)
                .context("blob base fee update fraction exceeds u64")?,
        })
    } else {
        None
    };

    Ok(ForkConfig::new(
        fork,
        ForkActivation::new(at_block, at_time),
        blob_schedule,
    ))
}

fn schedule_key(fork: ProtocolFork) -> Option<&'static str> {
    Some(match fork {
        // The capitalized key matches the spelling that
        // `blob_schedule_blob_params` of alloy-genesis expects, where every
        // other fork key is lowercase.
        ProtocolFork::Amsterdam => "Amsterdam",
        ProtocolFork::BPO2 => "bpo2",
        ProtocolFork::BPO1 => "bpo1",
        ProtocolFork::Osaka => "osaka",
        ProtocolFork::Prague => "prague",
        ProtocolFork::Cancun => "cancun",
        _ => return None,
    })
}

/// Converts a [`reth_stateless::StatelessInput`] to the [`NewPayloadRequest`]
/// container shape of the given protocol fork, following the partition of
/// [`NewPayloadRequest::matches_fork`]. Forks before Paris have no canonical
/// payload representation and fail the conversion.
fn to_new_payload_request(
    stateless_input: &reth_stateless::StatelessInput,
    requests: Requests,
    fork: ProtocolFork,
) -> Result<NewPayloadRequest> {
    let header = stateless_input.block.header();
    let body = stateless_input.block.body();

    let parent_hash = header.parent_hash.0;
    let fee_recipient = header.beneficiary.0.0;
    let state_root = header.state_root.0;
    let receipts_root = header.receipts_root.0;
    let logs_bloom = *header.logs_bloom.data();
    let prev_randao = header.mix_hash.0;
    let block_number = header.number;
    let gas_limit = header.gas_limit;
    let gas_used = header.gas_used;
    let timestamp = header.timestamp;
    let extra_data = ssz_list(header.extra_data.to_vec(), "extra data")?;
    let base_fee_per_gas = U256::from(header.base_fee_per_gas.unwrap_or_default()).to_le_bytes();
    let block_hash = stateless_input.block.hash_slow().0;
    let transactions = ssz_list(
        body.transactions()
            .map(|tx| ssz_list(tx.encoded_2718(), "transaction"))
            .collect::<Result<Vec<_>>>()?,
        "transactions",
    )?;
    let withdrawals = ssz_list(
        body.withdrawals
            .as_ref()
            .map(|withdrawals| {
                withdrawals
                    .iter()
                    .map(|withdrawal| Withdrawal {
                        index: withdrawal.index,
                        validator_index: withdrawal.validator_index,
                        address: withdrawal.address.0.0,
                        amount: withdrawal.amount,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        "withdrawals",
    )?;
    let blob_gas_used = header.blob_gas_used.unwrap_or_default();
    let excess_blob_gas = header.excess_blob_gas.unwrap_or_default();

    let versioned_hashes = ssz_list(
        body.transactions()
            .filter_map(|tx| tx.blob_versioned_hashes())
            .flatten()
            .map(|hash| hash.0)
            .collect::<Vec<_>>(),
        "versioned hashes",
    )?;
    let parent_beacon_block_root = stateless_input
        .block
        .parent_beacon_block_root
        .unwrap_or_default()
        .0;
    let execution_requests = decode_execution_requests(&requests)?;

    match fork {
        ProtocolFork::Paris => Ok(NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix {
            execution_payload: ExecutionPayloadV1 {
                parent_hash,
                fee_recipient,
                state_root,
                receipts_root,
                logs_bloom,
                prev_randao,
                block_number,
                gas_limit,
                gas_used,
                timestamp,
                extra_data,
                base_fee_per_gas,
                block_hash,
                transactions,
            },
        })),
        ProtocolFork::Shanghai => Ok(NewPayloadRequest::Capella(NewPayloadRequestCapella {
            execution_payload: ExecutionPayloadV2 {
                parent_hash,
                fee_recipient,
                state_root,
                receipts_root,
                logs_bloom,
                prev_randao,
                block_number,
                gas_limit,
                gas_used,
                timestamp,
                extra_data,
                base_fee_per_gas,
                block_hash,
                transactions,
                withdrawals,
            },
        })),
        ProtocolFork::Cancun => Ok(NewPayloadRequest::Deneb(NewPayloadRequestDeneb {
            execution_payload: ExecutionPayloadV3 {
                parent_hash,
                fee_recipient,
                state_root,
                receipts_root,
                logs_bloom,
                prev_randao,
                block_number,
                gas_limit,
                gas_used,
                timestamp,
                extra_data,
                base_fee_per_gas,
                block_hash,
                transactions,
                withdrawals,
                blob_gas_used,
                excess_blob_gas,
            },
            versioned_hashes,
            parent_beacon_block_root,
        })),
        ProtocolFork::Prague | ProtocolFork::Osaka | ProtocolFork::BPO1 | ProtocolFork::BPO2 => {
            let execution_payload = ExecutionPayloadV3 {
                parent_hash,
                fee_recipient,
                state_root,
                receipts_root,
                logs_bloom,
                prev_randao,
                block_number,
                gas_limit,
                gas_used,
                timestamp,
                extra_data,
                base_fee_per_gas,
                block_hash,
                transactions,
                withdrawals,
                blob_gas_used,
                excess_blob_gas,
            };
            Ok(NewPayloadRequest::ElectraFulu(
                NewPayloadRequestElectraFulu {
                    execution_payload,
                    versioned_hashes,
                    parent_beacon_block_root,
                    execution_requests,
                },
            ))
        }
        ProtocolFork::Amsterdam => Ok(NewPayloadRequest::Gloas(NewPayloadRequestGloas {
            execution_payload: ExecutionPayloadV4 {
                parent_hash,
                fee_recipient,
                state_root,
                receipts_root,
                logs_bloom,
                prev_randao,
                block_number,
                gas_limit,
                gas_used,
                timestamp,
                extra_data,
                base_fee_per_gas,
                block_hash,
                transactions,
                withdrawals,
                blob_gas_used,
                excess_blob_gas,
                // TODO(Amsterdam): Use the actual block access list.
                block_access_list: Default::default(),
                slot_number: header.slot_number.unwrap_or_default(),
            },
            versioned_hashes,
            parent_beacon_block_root,
            execution_requests,
        })),
        fork => anyhow::bail!("protocol fork {fork:?} has no canonical payload representation"),
    }
}

/// Decodes EIP-7685 encoded execution requests into an [`ExecutionRequests`] container.
///
/// Each request is a one byte request type followed by the request data. Type 0x00 carries
/// EIP-6110 deposit requests, type 0x01 carries EIP-7002 withdrawal requests, and type 0x02
/// carries EIP-7251 consolidation requests.
fn decode_execution_requests(requests_list: &[impl AsRef<[u8]>]) -> Result<ExecutionRequests> {
    const DEPOSIT_REQUEST_TYPE: u8 = 0x00;
    const WITHDRAWAL_REQUEST_TYPE: u8 = 0x01;
    const CONSOLIDATION_REQUEST_TYPE: u8 = 0x02;

    let mut deposits = Vec::new();
    let mut withdrawals = Vec::new();
    let mut consolidations = Vec::new();

    let mut last_request_type: Option<u8> = None;

    for (idx, request) in requests_list.iter().enumerate() {
        let request_bytes = request.as_ref();

        anyhow::ensure!(!request_bytes.is_empty(), "Empty request at index {}", idx);

        let request_type = request_bytes[0];
        let data = &request_bytes[1..];

        // EIP-7685 requires request types to be unique and ascending.
        if let Some(last_type) = last_request_type {
            anyhow::ensure!(
                request_type > last_type,
                "Invalid request ordering at index {}: type {:#x} must be greater than previous type {:#x}",
                idx,
                request_type,
                last_type
            );
        }
        last_request_type = Some(request_type);

        match request_type {
            DEPOSIT_REQUEST_TYPE => {
                deposits.extend(decode_typed_requests::<DepositRequest>(
                    data, "deposit", idx,
                )?);
            }
            WITHDRAWAL_REQUEST_TYPE => {
                withdrawals.extend(decode_typed_requests::<WithdrawalRequest>(
                    data,
                    "withdrawal",
                    idx,
                )?);
            }
            CONSOLIDATION_REQUEST_TYPE => {
                consolidations.extend(decode_typed_requests::<ConsolidationRequest>(
                    data,
                    "consolidation",
                    idx,
                )?);
            }
            _ => {
                anyhow::bail!("Unknown request type at index {}: {:#x}", idx, request_type);
            }
        }
    }

    Ok(ExecutionRequests {
        deposits: ssz_list(deposits, "deposits")?,
        withdrawals: ssz_list(withdrawals, "withdrawals")?,
        consolidations: ssz_list(consolidations, "consolidations")?,
    })
}

/// Decodes the concatenated fixed-size SSZ requests of one EIP-7685 request
/// type.
fn decode_typed_requests<T: SszDecode + SszEncode>(
    data: &[u8],
    label: &str,
    idx: usize,
) -> Result<Vec<T>> {
    let request_size = <T as SszEncode>::fixed_size();
    anyhow::ensure!(
        data.len().is_multiple_of(request_size),
        "{label} request data length {} is not a multiple of {} at index {}",
        data.len(),
        request_size,
        idx
    );
    data.chunks_exact(request_size)
        .enumerate()
        .map(|(i, chunk)| {
            T::from_ssz_bytes(chunk).map_err(|err| {
                anyhow::anyhow!("Failed to SSZ decode {label} request {i} at index {idx}: {err:?}")
            })
        })
        .collect()
}

/// Converts a legacy execution witness into the canonical witness. The
/// canonical witness has no keys field because stateless validation does not
/// consume it.
fn to_canonical_witness(witness: &reth_stateless::ExecutionWitness) -> Result<ExecutionWitness> {
    Ok(ExecutionWitness {
        state: ssz_bytes_list(&witness.state, "witness state")?,
        codes: ssz_bytes_list(&witness.codes, "witness codes")?,
        headers: ssz_bytes_list(&witness.headers, "witness headers")?,
    })
}

fn ssz_bytes_list<const M: usize, const N: usize>(
    items: &[alloy_primitives::Bytes],
    label: &str,
) -> Result<SszList<SszList<u8, M>, N>> {
    let list = items
        .iter()
        .map(|item| SszList::try_from(item.to_vec()))
        .collect::<Result<_, _>>()
        .map_err(|err| anyhow::anyhow!("{label} item length should be within bounds: {err:?}"))?;
    ssz_list(list, label)
}

fn ssz_list<T, const N: usize>(values: Vec<T>, label: &str) -> Result<SszList<T, N>> {
    SszList::try_from(values)
        .map_err(|err| anyhow::anyhow!("{label} length should be within bounds: {err:?}"))
}

#[cfg(test)]
mod tests {
    use crate::{guest::input::ProtocolFork, host::legacy::to_canonical_chain_config};

    /// Mainnet style configuration that merged through terminal total
    /// difficulty and carries no netsplit block.
    fn merged_config() -> alloy_genesis::ChainConfig {
        alloy_genesis::ChainConfig {
            chain_id: 1,
            homestead_block: Some(0),
            eip150_block: Some(0),
            eip155_block: Some(0),
            eip158_block: Some(0),
            byzantium_block: Some(0),
            constantinople_block: Some(0),
            petersburg_block: Some(0),
            istanbul_block: Some(0),
            muir_glacier_block: Some(0),
            berlin_block: Some(0),
            london_block: Some(0),
            arrow_glacier_block: Some(0),
            gray_glacier_block: Some(0),
            terminal_total_difficulty_passed: true,
            ..Default::default()
        }
    }

    #[test]
    fn paris_activates_through_terminal_total_difficulty() {
        let canonical = to_canonical_chain_config(&merged_config(), 15_537_394, 0).unwrap();

        assert_eq!(canonical.active_fork.fork, ProtocolFork::Paris);
        assert_eq!(canonical.active_fork.activation.block_number(), Some(0));
    }

    #[test]
    fn merge_netsplit_block_pins_the_paris_activation() {
        let config = alloy_genesis::ChainConfig {
            merge_netsplit_block: Some(15_537_394),
            ..merged_config()
        };

        let canonical = to_canonical_chain_config(&config, 15_537_394, 0).unwrap();

        assert_eq!(canonical.active_fork.fork, ProtocolFork::Paris);
        assert_eq!(
            canonical.active_fork.activation.block_number(),
            Some(15_537_394)
        );
    }

    #[test]
    fn scheduled_bpo3_to_bpo5_fail_the_conversion() {
        let config = alloy_genesis::ChainConfig {
            shanghai_time: Some(0),
            bpo3_time: Some(1_000),
            amsterdam_time: Some(2_000),
            ..merged_config()
        };

        assert!(to_canonical_chain_config(&config, 25_000_000, 2_500).is_err());
    }

    #[test]
    fn time_forks_win_over_the_merge() {
        let config = alloy_genesis::ChainConfig {
            shanghai_time: Some(100),
            ..merged_config()
        };

        let canonical = to_canonical_chain_config(&config, 17_034_870, 100).unwrap();

        assert_eq!(canonical.active_fork.fork, ProtocolFork::Shanghai);
        assert_eq!(canonical.active_fork.activation.timestamp(), Some(100));
    }
}
