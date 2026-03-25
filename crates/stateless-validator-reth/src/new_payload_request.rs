//! Execution payload request to block utilities.

use alloc::{sync::Arc, vec::Vec};

use alloy_consensus::Block;
use alloy_eips::eip4895::Withdrawal as AlloyWithdrawal;
use alloy_primitives::{Address, B256, Bloom, Bytes, U256};
use alloy_rpc_types_engine::{
    CancunPayloadFields, ExecutionData, ExecutionPayload as AlloyExecutionPayload,
    ExecutionPayloadSidecar, ExecutionPayloadV1 as AlloyExecutionPayloadV1,
    ExecutionPayloadV2 as AlloyExecutionPayloadV2, ExecutionPayloadV3 as AlloyExecutionPayloadV3,
    PayloadError,
};
use anyhow::{Context, Result};
use reth_chainspec::{ChainSpec, EthereumHardforks};
use reth_payload_validator::{cancun, prague, shanghai};
use reth_primitives_traits::{Block as _, SealedBlock, SignedTransaction};
use stateless_validator_common::new_payload_request::{
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, NativeSha256Hasher,
    NewPayloadRequest, Withdrawal, compute_requests_hash,
};

/// Converts a [`NewPayloadRequest`] into a validated reth [`Block`].
pub fn new_payload_request_to_block(
    new_payload_request: NewPayloadRequest,
    chain_spec: Arc<ChainSpec>,
) -> Result<SealedBlock<Block<reth_ethereum_primitives::TransactionSigned>>> {
    let execution_data = new_payload_request_to_execution_data(new_payload_request);
    let sealed_block = ensure_well_formed_payload(chain_spec, execution_data)
        .context("Payload validation failed")?;
    Ok(sealed_block)
}

/// This method is copied from `reth-ethereum-payload-builder` crate.
/// https://github.com/paradigmxyz/reth/blob/8eecad3d1d433ed509373713c21c31504290d17d/crates/ethereum/payload/src/validator.rs#L66
/// Unfortunately, that crate does not allow to have minimal activated
/// features in alloy-consensus to use directly without pulling in blst
/// and other non-friendly dependencies for zkVMs.
/// TODO: If we can upstream some changes to this crate accordingly, we
/// can remove this method and use directly from reth.
fn ensure_well_formed_payload<ChainSpec, T>(
    chain_spec: ChainSpec,
    payload: ExecutionData,
) -> Result<SealedBlock<Block<T>>, PayloadError>
where
    ChainSpec: EthereumHardforks,
    T: SignedTransaction,
{
    let ExecutionData { payload, sidecar } = payload;

    let expected_hash = payload.block_hash();

    // First parse the block
    let sealed_block = payload.try_into_block_with_sidecar(&sidecar)?.seal_slow();

    // Ensure the hash included in the payload matches the block hash
    if expected_hash != sealed_block.hash() {
        return Err(PayloadError::BlockHash {
            execution: sealed_block.hash(),
            consensus: expected_hash,
        });
    }

    shanghai::ensure_well_formed_fields(
        sealed_block.body(),
        chain_spec.is_shanghai_active_at_timestamp(sealed_block.timestamp),
    )?;

    cancun::ensure_well_formed_fields(
        &sealed_block,
        sidecar.cancun(),
        chain_spec.is_cancun_active_at_timestamp(sealed_block.timestamp),
    )?;

    prague::ensure_well_formed_fields(
        sealed_block.body(),
        sidecar.prague(),
        chain_spec.is_prague_active_at_timestamp(sealed_block.timestamp),
    )?;

    Ok(sealed_block)
}

fn new_payload_request_to_execution_data(req: NewPayloadRequest) -> ExecutionData {
    match req {
        NewPayloadRequest::Bellatrix(b) => {
            let v1 = convert_v1_to_alloy(b.execution_payload);
            ExecutionData::new(
                AlloyExecutionPayload::V1(v1),
                ExecutionPayloadSidecar::none(),
            )
        }
        NewPayloadRequest::Capella(c) => {
            let (v1, withdrawals) = convert_v2_to_alloy(c.execution_payload);
            let v2 = AlloyExecutionPayloadV2 {
                payload_inner: v1,
                withdrawals,
            };
            ExecutionData::new(
                AlloyExecutionPayload::V2(v2),
                ExecutionPayloadSidecar::none(),
            )
        }
        NewPayloadRequest::Deneb(d) => {
            let blob_gas_used = d.execution_payload.blob_gas_used;
            let excess_blob_gas = d.execution_payload.excess_blob_gas;
            let (v1, withdrawals) = convert_v2_to_alloy_from_v3(d.execution_payload);
            let v3 = AlloyExecutionPayloadV3 {
                payload_inner: AlloyExecutionPayloadV2 {
                    payload_inner: v1,
                    withdrawals,
                },
                blob_gas_used,
                excess_blob_gas,
            };

            let versioned_hashes: Vec<B256> =
                d.versioned_hashes.into_iter().map(B256::from).collect();
            let parent_beacon_block_root = B256::from(d.parent_beacon_block_root);
            let cancun_fields =
                CancunPayloadFields::new(parent_beacon_block_root, versioned_hashes);
            let sidecar = ExecutionPayloadSidecar::v3(cancun_fields);

            ExecutionData::new(AlloyExecutionPayload::V3(v3), sidecar)
        }
        NewPayloadRequest::ElectraFulu(e) => {
            let blob_gas_used = e.execution_payload.blob_gas_used;
            let excess_blob_gas = e.execution_payload.excess_blob_gas;
            let (v1, withdrawals) = convert_v2_to_alloy_from_v3(e.execution_payload);
            let v3 = AlloyExecutionPayloadV3 {
                payload_inner: AlloyExecutionPayloadV2 {
                    payload_inner: v1,
                    withdrawals,
                },
                blob_gas_used,
                excess_blob_gas,
            };

            let versioned_hashes: Vec<B256> =
                e.versioned_hashes.into_iter().map(B256::from).collect();
            let parent_beacon_block_root = B256::from(e.parent_beacon_block_root);
            let cancun_fields =
                CancunPayloadFields::new(parent_beacon_block_root, versioned_hashes);

            let requests_hash = B256::from(compute_requests_hash(
                &e.execution_requests,
                &NativeSha256Hasher,
            ));
            let prague_fields = alloy_rpc_types_engine::PraguePayloadFields::new(requests_hash);
            let sidecar = ExecutionPayloadSidecar::v4(cancun_fields, prague_fields);

            ExecutionData::new(AlloyExecutionPayload::V3(v3), sidecar)
        }
    }
}

fn convert_v1_to_alloy(payload: ExecutionPayloadV1) -> AlloyExecutionPayloadV1 {
    AlloyExecutionPayloadV1 {
        parent_hash: B256::from(payload.parent_hash),
        fee_recipient: Address::from(payload.fee_recipient),
        state_root: B256::from(payload.state_root),
        receipts_root: B256::from(payload.receipts_root),
        logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
        prev_randao: B256::from(payload.prev_randao),
        block_number: payload.block_number,
        gas_limit: payload.gas_limit,
        gas_used: payload.gas_used,
        timestamp: payload.timestamp,
        extra_data: Bytes::from(payload.extra_data.into_inner()),
        base_fee_per_gas: U256::from_le_bytes(payload.base_fee_per_gas),
        block_hash: B256::from(payload.block_hash),
        transactions: payload
            .transactions
            .into_iter()
            .map(|tx| Bytes::from(tx.into_inner()))
            .collect(),
    }
}

fn convert_v2_to_alloy(
    payload: ExecutionPayloadV2,
) -> (AlloyExecutionPayloadV1, Vec<AlloyWithdrawal>) {
    let v1 = AlloyExecutionPayloadV1 {
        parent_hash: B256::from(payload.parent_hash),
        fee_recipient: Address::from(payload.fee_recipient),
        state_root: B256::from(payload.state_root),
        receipts_root: B256::from(payload.receipts_root),
        logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
        prev_randao: B256::from(payload.prev_randao),
        block_number: payload.block_number,
        gas_limit: payload.gas_limit,
        gas_used: payload.gas_used,
        timestamp: payload.timestamp,
        extra_data: Bytes::from(payload.extra_data.into_inner()),
        base_fee_per_gas: U256::from_le_bytes(payload.base_fee_per_gas),
        block_hash: B256::from(payload.block_hash),
        transactions: payload
            .transactions
            .into_iter()
            .map(|tx| Bytes::from(tx.into_inner()))
            .collect(),
    };

    let withdrawals = payload
        .withdrawals
        .into_iter()
        .map(convert_withdrawal)
        .collect();

    (v1, withdrawals)
}

fn convert_v2_to_alloy_from_v3(
    payload: ExecutionPayloadV3,
) -> (AlloyExecutionPayloadV1, Vec<AlloyWithdrawal>) {
    let v1 = AlloyExecutionPayloadV1 {
        parent_hash: B256::from(payload.parent_hash),
        fee_recipient: Address::from(payload.fee_recipient),
        state_root: B256::from(payload.state_root),
        receipts_root: B256::from(payload.receipts_root),
        logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
        prev_randao: B256::from(payload.prev_randao),
        block_number: payload.block_number,
        gas_limit: payload.gas_limit,
        gas_used: payload.gas_used,
        timestamp: payload.timestamp,
        extra_data: Bytes::from(payload.extra_data.into_inner()),
        base_fee_per_gas: U256::from_le_bytes(payload.base_fee_per_gas),
        block_hash: B256::from(payload.block_hash),
        transactions: payload
            .transactions
            .into_iter()
            .map(|tx| Bytes::from(tx.into_inner()))
            .collect(),
    };

    let withdrawals = payload
        .withdrawals
        .into_iter()
        .map(convert_withdrawal)
        .collect();

    (v1, withdrawals)
}

fn convert_withdrawal(w: Withdrawal) -> AlloyWithdrawal {
    AlloyWithdrawal {
        index: w.index,
        validator_index: w.validator_index,
        address: Address::from(w.address),
        amount: w.amount,
    }
}
