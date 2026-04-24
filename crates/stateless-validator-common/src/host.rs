//! Stateless validator common types and utilities for host.

use anyhow::{Context, Result};
use libssz::{SszDecode, SszEncode};
use libssz_types::SszList;
use sha2::{Digest, Sha256};

use crate::{
    guest::StatelessValidatorOutput,
    new_payload_request::{
        ConsolidationRequest, DepositRequest, ExecutionPayloadV1, ExecutionPayloadV2,
        ExecutionPayloadV3, ExecutionPayloadV4, ExecutionRequests, Hash32, NewPayloadRequest,
        NewPayloadRequestAmsterdam, NewPayloadRequestBellatrix, NewPayloadRequestCapella,
        NewPayloadRequestDeneb, NewPayloadRequestElectraFulu, WithdrawalRequest,
    },
};

impl StatelessValidatorOutput {
    /// Returns sha256 digest of serialized output.
    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(self.serialize()).into()
    }
}

impl NewPayloadRequest {
    /// Constructs a new [`NewPayloadRequest`] for Bellatrix.
    pub fn new_bellatrix(execution_payload: ExecutionPayloadV1) -> Self {
        NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix { execution_payload })
    }

    /// Constructs a new [`NewPayloadRequest`] for Capella.
    pub fn new_capella(execution_payload: ExecutionPayloadV2) -> Self {
        NewPayloadRequest::Capella(NewPayloadRequestCapella { execution_payload })
    }

    /// Constructs a new [`NewPayloadRequest`] for Deneb.
    pub fn new_deneb(
        execution_payload: ExecutionPayloadV3,
        versioned_hashes: Vec<Hash32>,
        parent_beacon_block_root: Hash32,
    ) -> Result<Self> {
        let versioned_hashes = bounded_list(versioned_hashes, "versioned hashes")?;
        Ok(NewPayloadRequest::Deneb(NewPayloadRequestDeneb {
            execution_payload,
            versioned_hashes,
            parent_beacon_block_root,
        }))
    }

    /// Constructs a new [`NewPayloadRequest`] for Electra or Fulu.
    pub fn new_electra_fulu(
        execution_payload: ExecutionPayloadV3,
        versioned_hashes: Vec<Hash32>,
        parent_beacon_block_root: Hash32,
        execution_requests: &[impl AsRef<[u8]>],
    ) -> Result<Self> {
        let versioned_hashes = bounded_list(versioned_hashes, "versioned hashes")?;
        let execution_requests = decode_execution_requests(execution_requests)
            .context("Decoding execution requests failed")?;
        Ok(NewPayloadRequest::ElectraFulu(
            NewPayloadRequestElectraFulu {
                execution_payload,
                versioned_hashes,
                parent_beacon_block_root,
                execution_requests,
            },
        ))
    }

    /// Constructs a new [`NewPayloadRequest`] for Amsterdam.
    pub fn new_amsterdam(
        execution_payload: ExecutionPayloadV4,
        versioned_hashes: Vec<Hash32>,
        parent_beacon_block_root: Hash32,
        execution_requests: &[impl AsRef<[u8]>],
    ) -> Result<Self> {
        let versioned_hashes = bounded_list(versioned_hashes, "versioned hashes")?;
        let execution_requests = decode_execution_requests(execution_requests)
            .context("Decoding execution requests failed")?;
        Ok(NewPayloadRequest::Amsterdam(NewPayloadRequestAmsterdam {
            execution_payload,
            versioned_hashes,
            parent_beacon_block_root,
            execution_requests,
        }))
    }
}

fn bounded_list<T, const N: usize>(values: Vec<T>, label: &str) -> Result<SszList<T, N>> {
    SszList::try_from(values)
        .map_err(|err| anyhow::anyhow!("{label} length should be within bounds: {err:?}"))
}

/// Decodes a list of execution requests obtained from execution and deserializes them into an
/// [`ExecutionRequests`] struct.
fn decode_execution_requests(requests_list: &[impl AsRef<[u8]>]) -> Result<ExecutionRequests> {
    // EIP-7685: requests are encoded as request_type (1 byte) ++ request_data
    // Request types for Electra (Prague):
    // - 0x00: Deposit requests (EIP-6110)
    // - 0x01: Withdrawal requests (EIP-7002)
    // - 0x02: Consolidation requests (EIP-7251)

    const DEPOSIT_REQUEST_TYPE: u8 = 0x00;
    const WITHDRAWAL_REQUEST_TYPE: u8 = 0x01;
    const CONSOLIDATION_REQUEST_TYPE: u8 = 0x02;

    // Fixed SSZ sizes for each request type (excluding the type byte)
    let deposit_request_size = <DepositRequest as SszEncode>::fixed_size();
    let withdrawal_request_size = <WithdrawalRequest as SszEncode>::fixed_size();
    let consolidation_request_size = <ConsolidationRequest as SszEncode>::fixed_size();

    let mut deposits = Vec::new();
    let mut withdrawals = Vec::new();
    let mut consolidations = Vec::new();

    let mut last_request_type: Option<u8> = None;

    for (idx, request) in requests_list.iter().enumerate() {
        let request_bytes = request.as_ref();

        anyhow::ensure!(!request_bytes.is_empty(), "Empty request at index {}", idx);

        // Read request type (first byte)
        let request_type = request_bytes[0];
        let data = &request_bytes[1..];

        // Validate uniqueness and ascending order
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
                anyhow::ensure!(
                    data.len() % deposit_request_size == 0,
                    "Deposit request data length {} is not a multiple of {} at index {}",
                    data.len(),
                    deposit_request_size,
                    idx
                );

                for (i, chunk) in data.chunks_exact(deposit_request_size).enumerate() {
                    let deposit = DepositRequest::from_ssz_bytes(chunk).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to SSZ decode deposit request {} at index {}: {:?}",
                            i,
                            idx,
                            e
                        )
                    })?;
                    deposits.push(deposit);
                }
            }
            WITHDRAWAL_REQUEST_TYPE => {
                anyhow::ensure!(
                    data.len() % withdrawal_request_size == 0,
                    "Withdrawal request data length {} is not a multiple of {} at index {}",
                    data.len(),
                    withdrawal_request_size,
                    idx
                );

                for (i, chunk) in data.chunks_exact(withdrawal_request_size).enumerate() {
                    let withdrawal = WithdrawalRequest::from_ssz_bytes(chunk).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to SSZ decode withdrawal request {} at index {}: {:?}",
                            i,
                            idx,
                            e
                        )
                    })?;
                    withdrawals.push(withdrawal);
                }
            }
            CONSOLIDATION_REQUEST_TYPE => {
                anyhow::ensure!(
                    data.len() % consolidation_request_size == 0,
                    "Consolidation request data length {} is not a multiple of {} at index {}",
                    data.len(),
                    consolidation_request_size,
                    idx
                );

                for (i, chunk) in data.chunks_exact(consolidation_request_size).enumerate() {
                    let consolidation =
                        ConsolidationRequest::from_ssz_bytes(chunk).map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to SSZ decode consolidation request {} at index {}: {:?}",
                                i,
                                idx,
                                e
                            )
                        })?;
                    consolidations.push(consolidation);
                }
            }
            _ => {
                anyhow::bail!("Unknown request type at index {}: {:#x}", idx, request_type);
            }
        }
    }

    Ok(ExecutionRequests {
        deposits: bounded_list(deposits, "deposits")?,
        withdrawals: bounded_list(withdrawals, "withdrawals")?,
        consolidations: bounded_list(consolidations, "consolidations")?,
    })
}
