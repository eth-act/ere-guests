//! Conversion utilities for NewPayloadRequest to ethrex Block.

use anyhow::{Context, Result};
use ethrex_common::{Address, Bloom, Bytes, H256, types::Block};
use libssz_merkle::Sha256Hasher;
use stateless_validator_common::new_payload_request::{
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, NewPayloadRequest, Withdrawal,
    compute_requests_hash,
};

use crate::execution_payload::{
    EncodedTransaction, ExecutionPayload, validate_block_payload_v1_v2, validate_block_payload_v3,
    validate_execution_payload_v1, validate_execution_payload_v2, validate_execution_payload_v3,
};

/// Converts a [`NewPayloadRequest`] into an ethrex [`Block`].
pub fn get_block_from_new_payload_request(
    req: NewPayloadRequest,
    hasher: &impl Sha256Hasher,
) -> Result<Block> {
    match req {
        NewPayloadRequest::Bellatrix(b) => {
            let payload: ExecutionPayload = b.execution_payload.into();
            validate_execution_payload_v1(&payload).context("V1 payload validation failed")?;
            let block = payload
                .clone()
                .into_block(None, None)
                .context("into_block failed")?;
            validate_block_payload_v1_v2(&payload, &block)
                .context("Block/Payload validation failed")?;
            Ok(block)
        }
        NewPayloadRequest::Capella(c) => {
            let payload: ExecutionPayload = c.execution_payload.into();
            validate_execution_payload_v2(&payload).context("V2 payload validation failed")?;
            let block = payload
                .clone()
                .into_block(None, None)
                .context("into_block failed")?;
            validate_block_payload_v1_v2(&payload, &block)
                .context("Block/Payload validation failed")?;
            Ok(block)
        }
        NewPayloadRequest::Deneb(d) => {
            let parent_beacon_block_root = Some(H256::from(d.parent_beacon_block_root));
            let payload: ExecutionPayload = d.execution_payload.into();
            validate_execution_payload_v3(&payload).context("V3 payload validation failed")?;
            let block = payload
                .clone()
                .into_block(parent_beacon_block_root, None)
                .context("into_block failed")?;
            validate_block_payload_v3(&payload, &block, &d.versioned_hashes)
                .context("Block/Payload validation failed")?;
            Ok(block)
        }
        NewPayloadRequest::ElectraFulu(e) => {
            let parent_beacon_block_root = Some(H256::from(e.parent_beacon_block_root));
            let requests_hash = Some(H256::from(compute_requests_hash(
                &e.execution_requests,
                hasher,
            )));
            let payload: ExecutionPayload = e.execution_payload.into();
            validate_execution_payload_v3(&payload).context("V3 payload validation failed")?;
            let block = payload
                .clone()
                .into_block(parent_beacon_block_root, requests_hash)
                .context("into_block failed")?;
            validate_block_payload_v3(&payload, &block, &e.versioned_hashes)
                .context("Block/Payload validation failed")?;
            Ok(block)
        }
    }
}

impl From<ExecutionPayloadV1> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV1) -> Self {
        ExecutionPayload {
            parent_hash: H256::from(payload.parent_hash),
            fee_recipient: Address::from(payload.fee_recipient),
            state_root: H256::from(payload.state_root),
            receipts_root: H256::from(payload.receipts_root),
            logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
            prev_randao: H256::from(payload.prev_randao),
            block_number: payload.block_number,
            gas_limit: payload.gas_limit,
            gas_used: payload.gas_used,
            timestamp: payload.timestamp,
            extra_data: Bytes::from(payload.extra_data.into_inner()),
            base_fee_per_gas: base_fee_to_u64(&payload.base_fee_per_gas),
            block_hash: H256::from(payload.block_hash),
            transactions: payload
                .transactions
                .into_iter()
                .map(|t| EncodedTransaction(Bytes::from(t.into_inner())))
                .collect(),
            withdrawals: None,
            blob_gas_used: None,
            excess_blob_gas: None,
        }
    }
}

impl From<ExecutionPayloadV2> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV2) -> Self {
        ExecutionPayload {
            parent_hash: H256::from(payload.parent_hash),
            fee_recipient: Address::from(payload.fee_recipient),
            state_root: H256::from(payload.state_root),
            receipts_root: H256::from(payload.receipts_root),
            logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
            prev_randao: H256::from(payload.prev_randao),
            block_number: payload.block_number,
            gas_limit: payload.gas_limit,
            gas_used: payload.gas_used,
            timestamp: payload.timestamp,
            extra_data: Bytes::from(payload.extra_data.into_inner()),
            base_fee_per_gas: base_fee_to_u64(&payload.base_fee_per_gas),
            block_hash: H256::from(payload.block_hash),
            transactions: payload
                .transactions
                .into_iter()
                .map(|t| EncodedTransaction(Bytes::from(t.into_inner())))
                .collect(),
            withdrawals: Some(
                payload
                    .withdrawals
                    .into_iter()
                    .map(convert_withdrawal)
                    .collect(),
            ),
            blob_gas_used: None,
            excess_blob_gas: None,
        }
    }
}

impl From<ExecutionPayloadV3> for ExecutionPayload {
    fn from(payload: ExecutionPayloadV3) -> Self {
        ExecutionPayload {
            parent_hash: H256::from(payload.parent_hash),
            fee_recipient: Address::from(payload.fee_recipient),
            state_root: H256::from(payload.state_root),
            receipts_root: H256::from(payload.receipts_root),
            logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
            prev_randao: H256::from(payload.prev_randao),
            block_number: payload.block_number,
            gas_limit: payload.gas_limit,
            gas_used: payload.gas_used,
            timestamp: payload.timestamp,
            extra_data: Bytes::from(payload.extra_data.into_inner()),
            base_fee_per_gas: base_fee_to_u64(&payload.base_fee_per_gas),
            block_hash: H256::from(payload.block_hash),
            transactions: payload
                .transactions
                .into_iter()
                .map(|t| EncodedTransaction(Bytes::from(t.into_inner())))
                .collect(),
            withdrawals: Some(
                payload
                    .withdrawals
                    .into_iter()
                    .map(convert_withdrawal)
                    .collect(),
            ),
            blob_gas_used: Some(payload.blob_gas_used),
            excess_blob_gas: Some(payload.excess_blob_gas),
        }
    }
}

/// Convert our Withdrawal type to ethrex's Withdrawal type.
fn convert_withdrawal(w: Withdrawal) -> ethrex_common::types::Withdrawal {
    ethrex_common::types::Withdrawal {
        index: w.index,
        validator_index: w.validator_index,
        address: Address::from(w.address),
        amount: w.amount,
    }
}

/// Convert base_fee_per_gas from 32-byte little-endian to u64.
fn base_fee_to_u64(base_fee: &[u8; 32]) -> u64 {
    debug_assert!(base_fee[8..].iter().all(|&b| b == 0), "base_fee overflow");
    u64::from_le_bytes(base_fee[..8].try_into().unwrap())
}
