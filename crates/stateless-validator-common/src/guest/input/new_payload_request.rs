//! New payload request types and their dependencies, mirroring [`types.py`], [`requests.py`],
//! and [`blocks.py`].
//!
//! The execution payload containers keep the V1 to V4 names defined by the engine API in
//! execution-apis, because a multi-fork crate needs distinct names while each execution-specs
//! fork module defines a single `ExecutionPayload` shape.
//!
//! [`types.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/execution_engine/types.py
//! [`requests.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/execution_engine/requests.py
//! [`blocks.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.6.2/src/ethereum/forks/amsterdam/blocks.py

#![allow(missing_docs)]

use alloc::vec::Vec;

use libssz::{DecodeError, SszDecode, SszEncode};
use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};
use libssz_merkle::{HashTreeRoot, Sha256Hasher, mix_in_active_fields};
use libssz_types::SszList;

use crate::{ProgressiveList, merkleize_progressive};

/// Primitive types from the Amsterdam stateless schema.
pub type Hash32 = [u8; 32];
pub type Bytes48 = [u8; 48];
pub type Bytes96 = [u8; 96];
pub type Address = [u8; 20];
pub type Uint256Bytes = [u8; 32];
pub type Bloom = [u8; 256];
pub type VersionedHash = Hash32;
pub type ExtraData = SszList<u8, MAX_EXTRA_DATA_BYTES>;

/// SSZ list bounds from the Amsterdam stateless schema.
pub const MAX_EXTRA_DATA_BYTES: usize = 32;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1 << 20;
pub const MAX_BYTES_PER_TRANSACTION: usize = 1 << 30;
pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 8192;
pub const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 16;
pub const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 2;
pub const MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 64;
pub const MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD: usize = 16;
pub const MAX_BLOCK_ACCESS_LIST_BYTES: usize = 1 << 24;

/// Bounded composite types used before Gloas.
pub type Transaction = SszList<u8, MAX_BYTES_PER_TRANSACTION>;
pub type Transactions = SszList<Transaction, MAX_TRANSACTIONS_PER_PAYLOAD>;
pub type Withdrawals = SszList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>;
pub type VersionedHashes = SszList<VersionedHash, MAX_BLOB_COMMITMENTS_PER_BLOCK>;
pub type DepositRequests = SszList<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>;
pub type WithdrawalRequests = SszList<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>;
pub type ConsolidationRequests =
    SszList<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>;
pub type BuilderDepositRequests =
    SszList<BuilderDepositRequest, MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD>;
pub type BuilderExitRequests = SszList<BuilderExitRequest, MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD>;

/// Progressive composite types introduced by Gloas/EIP-7688.
pub type ProgressiveTransaction = ProgressiveList<u8>;
pub type ProgressiveTransactions = ProgressiveList<ProgressiveTransaction>;
pub type ProgressiveWithdrawals = ProgressiveList<Withdrawal>;
pub type BlockAccessList = ProgressiveList<u8>;
pub type ProgressiveDepositRequests = ProgressiveList<DepositRequest>;
pub type ProgressiveWithdrawalRequests = ProgressiveList<WithdrawalRequest>;
pub type ProgressiveConsolidationRequests = ProgressiveList<ConsolidationRequest>;
pub type ProgressiveBuilderDepositRequests = ProgressiveList<BuilderDepositRequest>;
pub type ProgressiveBuilderExitRequests = ProgressiveList<BuilderExitRequest>;

/// A borrowed view over bounded pre-Gloas or progressive Gloas transactions.
#[derive(Debug, Clone, Copy)]
pub enum TransactionsRef<'a> {
    Bounded(&'a Transactions),
    Progressive(&'a ProgressiveTransactions),
}

impl TransactionsRef<'_> {
    /// Returns the number of transactions.
    pub fn len(self) -> usize {
        match self {
            Self::Bounded(transactions) => transactions.len(),
            Self::Progressive(transactions) => transactions.len(),
        }
    }

    /// Returns whether there are no transactions.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

impl<'a> TransactionsRef<'a> {
    /// Iterates over transactions as byte slices.
    pub fn iter(self) -> TransactionsIter<'a> {
        match self {
            Self::Bounded(transactions) => TransactionsIter::Bounded(transactions.iter()),
            Self::Progressive(transactions) => TransactionsIter::Progressive(transactions.iter()),
        }
    }
}

impl<'a> IntoIterator for TransactionsRef<'a> {
    type Item = &'a [u8];
    type IntoIter = TransactionsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over either bounded pre-Gloas or progressive Gloas transactions.
#[derive(Debug)]
pub enum TransactionsIter<'a> {
    Bounded(core::slice::Iter<'a, Transaction>),
    Progressive(core::slice::Iter<'a, ProgressiveTransaction>),
}

impl<'a> Iterator for TransactionsIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Bounded(iter) => iter.next().map(|transaction| &transaction[..]),
            Self::Progressive(iter) => iter.next().map(|transaction| &transaction[..]),
        }
    }
}

/// Withdrawals represent a transfer of ETH from the consensus layer (beacon chain) to the
/// execution layer, as validated by the consensus layer. Each withdrawal is listed in the block's
/// list of withdrawals.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct Withdrawal {
    /// The unique index of the withdrawal, incremented for each withdrawal processed.
    pub index: u64,
    /// The index of the validator on the consensus layer that is withdrawing.
    pub validator_index: u64,
    /// The execution-layer address receiving the withdrawn ETH.
    pub address: Address,
    /// The amount of ETH being withdrawn.
    pub amount: u64,
}

/// A single EIP-6110 deposit request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct DepositRequest {
    pub pubkey: Bytes48,
    pub withdrawal_credentials: Hash32,
    pub amount: u64,
    pub signature: Bytes96,
    pub index: u64,
}

/// A single EIP-7002 withdrawal request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct WithdrawalRequest {
    pub source_address: Address,
    pub validator_pubkey: Bytes48,
    pub amount: u64,
}

/// A single EIP-7251 consolidation request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ConsolidationRequest {
    pub source_address: Address,
    pub source_pubkey: Bytes48,
    pub target_pubkey: Bytes48,
}

/// A single EIP-8282 builder deposit request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct BuilderDepositRequest {
    pub pubkey: Bytes48,
    pub withdrawal_credentials: Hash32,
    pub amount: u64,
    pub signature: Bytes96,
}

/// A single EIP-8282 builder exit request.
#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct BuilderExitRequest {
    pub source_address: Address,
    pub pubkey: Bytes48,
}

/// Typed engine-API container of execution-layer triggered requests, as of Electra.
///
/// Mirrors the consensus-layer `ExecutionRequests` Container.
#[derive(Debug, Clone, Default, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionRequestsElectraFulu {
    pub deposits: DepositRequests,
    pub withdrawals: WithdrawalRequests,
    pub consolidations: ConsolidationRequests,
}

/// Typed engine-API container of execution-layer triggered requests, as of Gloas, which
/// EIP-8282 extends with the builder deposit and builder exit lists.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionRequestsGloas {
    pub deposits: ProgressiveDepositRequests,
    pub withdrawals: ProgressiveWithdrawalRequests,
    pub consolidations: ProgressiveConsolidationRequests,
    pub builder_deposits: ProgressiveBuilderDepositRequests,
    pub builder_exits: ProgressiveBuilderExitRequests,
}

impl HashTreeRoot for ExecutionRequestsGloas {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> [u8; 32] {
        progressive_container_root(
            hasher,
            &[
                self.deposits.hash_tree_root(hasher),
                self.withdrawals.hash_tree_root(hasher),
                self.consolidations.hash_tree_root(hasher),
                self.builder_deposits.hash_tree_root(hasher),
                self.builder_exits.hash_tree_root(hasher),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV1 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV2 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
    pub withdrawals: Withdrawals,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct ExecutionPayloadV3 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: Transactions,
    pub withdrawals: Withdrawals,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionPayloadV4 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    pub logs_bloom: Bloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    pub transactions: ProgressiveTransactions,
    pub withdrawals: ProgressiveWithdrawals,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
    pub block_access_list: BlockAccessList,
    pub slot_number: u64,
}

impl Default for ExecutionPayloadV4 {
    fn default() -> Self {
        Self {
            parent_hash: [0; 32],
            fee_recipient: [0; 20],
            state_root: [0; 32],
            receipts_root: [0; 32],
            logs_bloom: [0; 256],
            prev_randao: [0; 32],
            block_number: 0,
            gas_limit: 0,
            gas_used: 0,
            timestamp: 0,
            extra_data: Default::default(),
            base_fee_per_gas: [0; 32],
            block_hash: [0; 32],
            transactions: Default::default(),
            withdrawals: Default::default(),
            blob_gas_used: 0,
            excess_blob_gas: 0,
            block_access_list: Default::default(),
            slot_number: 0,
        }
    }
}

impl HashTreeRoot for ExecutionPayloadV4 {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> [u8; 32] {
        progressive_container_root(
            hasher,
            &[
                self.parent_hash.hash_tree_root(hasher),
                self.fee_recipient.hash_tree_root(hasher),
                self.state_root.hash_tree_root(hasher),
                self.receipts_root.hash_tree_root(hasher),
                self.logs_bloom.hash_tree_root(hasher),
                self.prev_randao.hash_tree_root(hasher),
                self.block_number.hash_tree_root(hasher),
                self.gas_limit.hash_tree_root(hasher),
                self.gas_used.hash_tree_root(hasher),
                self.timestamp.hash_tree_root(hasher),
                self.extra_data.hash_tree_root(hasher),
                self.base_fee_per_gas.hash_tree_root(hasher),
                self.block_hash.hash_tree_root(hasher),
                self.transactions.hash_tree_root(hasher),
                self.withdrawals.hash_tree_root(hasher),
                self.blob_gas_used.hash_tree_root(hasher),
                self.excess_blob_gas.hash_tree_root(hasher),
                self.block_access_list.hash_tree_root(hasher),
                self.slot_number.hash_tree_root(hasher),
            ],
        )
    }
}

fn progressive_container_root(hasher: &impl Sha256Hasher, field_roots: &[[u8; 32]]) -> [u8; 32] {
    let root = merkleize_progressive(hasher, field_roots);
    let active_fields = alloc::vec![true; field_roots.len()];
    mix_in_active_fields(hasher, &root, &active_fields)
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestBellatrix {
    pub execution_payload: ExecutionPayloadV1,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestCapella {
    pub execution_payload: ExecutionPayloadV2,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestDeneb {
    pub execution_payload: ExecutionPayloadV3,
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestElectraFulu {
    pub execution_payload: ExecutionPayloadV3,
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
    pub execution_requests: ExecutionRequestsElectraFulu,
}

#[derive(Debug, Clone, PartialEq, Eq, HashTreeRoot, SszEncode, SszDecode)]
pub struct NewPayloadRequestGloas {
    pub execution_payload: ExecutionPayloadV4,
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
    pub execution_requests: ExecutionRequestsGloas,
}

/// Consensus-layer new payload request with one container shape per fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewPayloadRequest {
    Bellatrix(NewPayloadRequestBellatrix),
    Capella(NewPayloadRequestCapella),
    Deneb(NewPayloadRequestDeneb),
    ElectraFulu(NewPayloadRequestElectraFulu),
    Gloas(NewPayloadRequestGloas),
}

impl NewPayloadRequest {
    /// Returns the block number of the execution payload.
    pub fn block_number(&self) -> u64 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.block_number,
            Self::Capella(request) => request.execution_payload.block_number,
            Self::Deneb(request) => request.execution_payload.block_number,
            Self::ElectraFulu(request) => request.execution_payload.block_number,
            Self::Gloas(request) => request.execution_payload.block_number,
        }
    }

    /// Returns the timestamp of the execution payload.
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.timestamp,
            Self::Capella(request) => request.execution_payload.timestamp,
            Self::Deneb(request) => request.execution_payload.timestamp,
            Self::ElectraFulu(request) => request.execution_payload.timestamp,
            Self::Gloas(request) => request.execution_payload.timestamp,
        }
    }

    /// Returns the gas used by the execution payload.
    pub fn gas_used(&self) -> u64 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.gas_used,
            Self::Capella(request) => request.execution_payload.gas_used,
            Self::Deneb(request) => request.execution_payload.gas_used,
            Self::ElectraFulu(request) => request.execution_payload.gas_used,
            Self::Gloas(request) => request.execution_payload.gas_used,
        }
    }

    /// Returns the block hash of the execution payload.
    pub fn block_hash(&self) -> Hash32 {
        match self {
            Self::Bellatrix(request) => request.execution_payload.block_hash,
            Self::Capella(request) => request.execution_payload.block_hash,
            Self::Deneb(request) => request.execution_payload.block_hash,
            Self::ElectraFulu(request) => request.execution_payload.block_hash,
            Self::Gloas(request) => request.execution_payload.block_hash,
        }
    }

    /// Returns the transactions of the execution payload.
    pub fn transactions(&self) -> TransactionsRef<'_> {
        match self {
            Self::Bellatrix(request) => {
                TransactionsRef::Bounded(&request.execution_payload.transactions)
            }
            Self::Capella(request) => {
                TransactionsRef::Bounded(&request.execution_payload.transactions)
            }
            Self::Deneb(request) => {
                TransactionsRef::Bounded(&request.execution_payload.transactions)
            }
            Self::ElectraFulu(request) => {
                TransactionsRef::Bounded(&request.execution_payload.transactions)
            }
            Self::Gloas(request) => {
                TransactionsRef::Progressive(&request.execution_payload.transactions)
            }
        }
    }

    /// Enforces the consensus maxima for Gloas progressive lists at runtime.
    pub fn validate_progressive_limits(&self) -> Result<(), crate::guest::Error> {
        let Self::Gloas(request) = self else {
            return Ok(());
        };
        let payload = &request.execution_payload;
        ensure_runtime_limit(
            "execution_payload.transactions",
            payload.transactions.len(),
            MAX_TRANSACTIONS_PER_PAYLOAD,
        )?;
        for transaction in &payload.transactions {
            ensure_runtime_limit(
                "execution_payload.transactions[]",
                transaction.len(),
                MAX_BYTES_PER_TRANSACTION,
            )?;
        }
        ensure_runtime_limit(
            "execution_payload.withdrawals",
            payload.withdrawals.len(),
            MAX_WITHDRAWALS_PER_PAYLOAD,
        )?;
        ensure_runtime_limit(
            "execution_payload.block_access_list",
            payload.block_access_list.len(),
            MAX_BLOCK_ACCESS_LIST_BYTES,
        )?;

        let requests = &request.execution_requests;
        ensure_runtime_limit(
            "execution_requests.deposits",
            requests.deposits.len(),
            MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
        )?;
        ensure_runtime_limit(
            "execution_requests.withdrawals",
            requests.withdrawals.len(),
            MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
        )?;
        ensure_runtime_limit(
            "execution_requests.consolidations",
            requests.consolidations.len(),
            MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
        )?;
        ensure_runtime_limit(
            "execution_requests.builder_deposits",
            requests.builder_deposits.len(),
            MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD,
        )?;
        ensure_runtime_limit(
            "execution_requests.builder_exits",
            requests.builder_exits.len(),
            MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD,
        )
    }
}

fn ensure_runtime_limit(
    field: &'static str,
    length: usize,
    max: usize,
) -> Result<(), crate::guest::Error> {
    if length > max {
        Err(crate::guest::Error::ProgressiveListTooLong { field, length, max })
    } else {
        Ok(())
    }
}

impl HashTreeRoot for NewPayloadRequest {
    fn hash_tree_root(&self, hasher: &impl Sha256Hasher) -> [u8; 32] {
        match self {
            Self::Bellatrix(request) => request.hash_tree_root(hasher),
            Self::Capella(request) => request.hash_tree_root(hasher),
            Self::Deneb(request) => request.hash_tree_root(hasher),
            Self::ElectraFulu(request) => request.hash_tree_root(hasher),
            Self::Gloas(request) => request.hash_tree_root(hasher),
        }
    }
}

impl SszEncode for NewPayloadRequest {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    fn encoded_len(&self) -> usize {
        match self {
            Self::Bellatrix(request) => request.encoded_len(),
            Self::Capella(request) => request.encoded_len(),
            Self::Deneb(request) => request.encoded_len(),
            Self::ElectraFulu(request) => request.encoded_len(),
            Self::Gloas(request) => request.encoded_len(),
        }
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Bellatrix(request) => request.ssz_append(buf),
            Self::Capella(request) => request.ssz_append(buf),
            Self::Deneb(request) => request.ssz_append(buf),
            Self::ElectraFulu(request) => request.ssz_append(buf),
            Self::Gloas(request) => request.ssz_append(buf),
        }
    }
}

impl SszDecode for NewPayloadRequest {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    /// Decodes by attempting each container shape from the newest fork to the
    /// oldest and returning the first success.
    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        NewPayloadRequestGloas::from_ssz_bytes(bytes)
            .map(Self::Gloas)
            .or_else(|_| NewPayloadRequestElectraFulu::from_ssz_bytes(bytes).map(Self::ElectraFulu))
            .or_else(|_| NewPayloadRequestDeneb::from_ssz_bytes(bytes).map(Self::Deneb))
            .or_else(|_| NewPayloadRequestCapella::from_ssz_bytes(bytes).map(Self::Capella))
            .or_else(|_| NewPayloadRequestBellatrix::from_ssz_bytes(bytes).map(Self::Bellatrix))
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use hex_literal::hex;
    use libssz_merkle::Sha2Hasher;

    use crate::guest::input::{
        ChainConfig, ExecutionWitness, ProtocolFork, StatelessInput, new_payload_request::*,
    };

    fn payload_v1() -> ExecutionPayloadV1 {
        ExecutionPayloadV1 {
            parent_hash: [1; 32],
            fee_recipient: [2; 20],
            state_root: [3; 32],
            receipts_root: [4; 32],
            logs_bloom: [5; 256],
            prev_randao: [6; 32],
            block_number: 100,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            timestamp: 1_700_000_000,
            extra_data: vec![0xee; 7].try_into().unwrap(),
            base_fee_per_gas: [7; 32],
            block_hash: [8; 32],
            transactions: vec![vec![0xdd_u8; 8].try_into().unwrap()]
                .try_into()
                .unwrap(),
        }
    }

    fn payload_v2() -> ExecutionPayloadV2 {
        let v1 = payload_v1();
        ExecutionPayloadV2 {
            parent_hash: v1.parent_hash,
            fee_recipient: v1.fee_recipient,
            state_root: v1.state_root,
            receipts_root: v1.receipts_root,
            logs_bloom: v1.logs_bloom,
            prev_randao: v1.prev_randao,
            block_number: v1.block_number,
            gas_limit: v1.gas_limit,
            gas_used: v1.gas_used,
            timestamp: v1.timestamp,
            extra_data: v1.extra_data,
            base_fee_per_gas: v1.base_fee_per_gas,
            block_hash: v1.block_hash,
            transactions: v1.transactions,
            withdrawals: vec![Withdrawal {
                index: 1,
                validator_index: 2,
                address: [3; 20],
                amount: 4,
            }]
            .try_into()
            .unwrap(),
        }
    }

    fn payload_v3() -> ExecutionPayloadV3 {
        let v2 = payload_v2();
        ExecutionPayloadV3 {
            parent_hash: v2.parent_hash,
            fee_recipient: v2.fee_recipient,
            state_root: v2.state_root,
            receipts_root: v2.receipts_root,
            logs_bloom: v2.logs_bloom,
            prev_randao: v2.prev_randao,
            block_number: v2.block_number,
            gas_limit: v2.gas_limit,
            gas_used: v2.gas_used,
            timestamp: v2.timestamp,
            extra_data: v2.extra_data,
            base_fee_per_gas: v2.base_fee_per_gas,
            block_hash: v2.block_hash,
            transactions: v2.transactions,
            withdrawals: v2.withdrawals,
            blob_gas_used: 131_072,
            excess_blob_gas: 262_144,
        }
    }

    fn payload_v4() -> ExecutionPayloadV4 {
        let v3 = payload_v3();
        ExecutionPayloadV4 {
            parent_hash: v3.parent_hash,
            fee_recipient: v3.fee_recipient,
            state_root: v3.state_root,
            receipts_root: v3.receipts_root,
            logs_bloom: v3.logs_bloom,
            prev_randao: v3.prev_randao,
            block_number: v3.block_number,
            gas_limit: v3.gas_limit,
            gas_used: v3.gas_used,
            timestamp: v3.timestamp,
            extra_data: v3.extra_data,
            base_fee_per_gas: v3.base_fee_per_gas,
            block_hash: v3.block_hash,
            transactions: v3
                .transactions
                .into_iter()
                .map(|transaction| ProgressiveTransaction::from(transaction.into_inner()))
                .collect::<Vec<_>>()
                .into(),
            withdrawals: v3.withdrawals.into_inner().into(),
            blob_gas_used: v3.blob_gas_used,
            excess_blob_gas: v3.excess_blob_gas,
            block_access_list: vec![0xba; 33].into(),
            slot_number: 42,
        }
    }

    fn versioned_hashes() -> VersionedHashes {
        vec![[9; 32]].try_into().unwrap()
    }

    fn deposit_requests() -> DepositRequests {
        vec![DepositRequest {
            pubkey: [1; 48],
            withdrawal_credentials: [2; 32],
            amount: 3,
            signature: [4; 96],
            index: 5,
        }]
        .try_into()
        .unwrap()
    }

    fn withdrawal_requests() -> WithdrawalRequests {
        vec![WithdrawalRequest {
            source_address: [1; 20],
            validator_pubkey: [2; 48],
            amount: 3,
        }]
        .try_into()
        .unwrap()
    }

    fn consolidation_requests() -> ConsolidationRequests {
        vec![ConsolidationRequest {
            source_address: [1; 20],
            source_pubkey: [2; 48],
            target_pubkey: [3; 48],
        }]
        .try_into()
        .unwrap()
    }

    fn builder_deposit_requests() -> BuilderDepositRequests {
        vec![BuilderDepositRequest {
            pubkey: [1; 48],
            withdrawal_credentials: [2; 32],
            amount: 3,
            signature: [4; 96],
        }]
        .try_into()
        .unwrap()
    }

    fn builder_exit_requests() -> BuilderExitRequests {
        vec![BuilderExitRequest {
            source_address: [1; 20],
            pubkey: [2; 48],
        }]
        .try_into()
        .unwrap()
    }

    fn execution_requests_electra_fulu() -> ExecutionRequestsElectraFulu {
        ExecutionRequestsElectraFulu {
            deposits: deposit_requests(),
            withdrawals: withdrawal_requests(),
            consolidations: consolidation_requests(),
        }
    }

    fn execution_requests_gloas() -> ExecutionRequestsGloas {
        ExecutionRequestsGloas {
            deposits: deposit_requests().into_inner().into(),
            withdrawals: withdrawal_requests().into_inner().into(),
            consolidations: consolidation_requests().into_inner().into(),
            builder_deposits: builder_deposit_requests().into_inner().into(),
            builder_exits: builder_exit_requests().into_inner().into(),
        }
    }

    fn bellatrix() -> NewPayloadRequest {
        NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix {
            execution_payload: payload_v1(),
        })
    }

    fn capella() -> NewPayloadRequest {
        NewPayloadRequest::Capella(NewPayloadRequestCapella {
            execution_payload: payload_v2(),
        })
    }

    fn deneb() -> NewPayloadRequest {
        NewPayloadRequest::Deneb(NewPayloadRequestDeneb {
            execution_payload: payload_v3(),
            versioned_hashes: versioned_hashes(),
            parent_beacon_block_root: [10; 32],
        })
    }

    fn electra_fulu() -> NewPayloadRequest {
        NewPayloadRequest::ElectraFulu(NewPayloadRequestElectraFulu {
            execution_payload: payload_v3(),
            versioned_hashes: versioned_hashes(),
            parent_beacon_block_root: [10; 32],
            execution_requests: execution_requests_electra_fulu(),
        })
    }

    fn gloas() -> NewPayloadRequest {
        NewPayloadRequest::Gloas(NewPayloadRequestGloas {
            execution_payload: payload_v4(),
            versioned_hashes: versioned_hashes(),
            parent_beacon_block_root: [10; 32],
            execution_requests: execution_requests_gloas(),
        })
    }

    fn stateless_input(new_payload_request: NewPayloadRequest) -> StatelessInput {
        StatelessInput {
            new_payload_request,
            witness: ExecutionWitness::default(),
            chain_config: ChainConfig::default(),
            public_keys: Default::default(),
        }
    }

    #[test]
    fn from_ssz_bytes_decodes_every_variant() {
        for request in [bellatrix(), capella(), deneb(), electra_fulu(), gloas()] {
            let decoded = NewPayloadRequest::from_ssz_bytes(&request.to_ssz()).unwrap();
            assert_eq!(decoded, request);
            assert_eq!(
                decoded.hash_tree_root(&Sha2Hasher),
                request.hash_tree_root(&Sha2Hasher)
            );
        }
    }

    #[test]
    fn matches_fork_partitions_every_variant_and_fork() {
        const ELECTRA_FULU_FORKS: &[ProtocolFork] = &[
            ProtocolFork::Prague,
            ProtocolFork::Osaka,
            ProtocolFork::BPO1,
            ProtocolFork::BPO2,
        ];
        for (request, matching) in [
            (bellatrix(), [ProtocolFork::Paris].as_slice()),
            (capella(), [ProtocolFork::Shanghai].as_slice()),
            (deneb(), [ProtocolFork::Cancun].as_slice()),
            (electra_fulu(), ELECTRA_FULU_FORKS),
            (gloas(), [ProtocolFork::Amsterdam].as_slice()),
        ] {
            for fork in 1..=ProtocolFork::Amsterdam.as_u8() {
                let fork = ProtocolFork::from_u8(fork).unwrap();
                let input = stateless_input(request.clone());
                let result =
                    StatelessInput::from_schema_prefixed_ssz(&input.to_schema_prefixed_ssz(fork));
                if matching.contains(&fork) {
                    let (decoded_fork, decoded) = result.unwrap();
                    assert_eq!(decoded_fork, fork);
                    assert_eq!(decoded.new_payload_request, request);
                } else {
                    assert!(result.is_err());
                }
            }
        }
    }

    #[test]
    fn gloas_progressive_roots_match_lighthouse_unstable() {
        // Generated with Lighthouse `unstable` at
        // e6a90c168436d8b8d6b5c779c9b0550bd56fb8c7.
        assert_eq!(
            ExecutionPayloadV4::default().hash_tree_root(&Sha2Hasher),
            hex!("19e3b044dae3657cb2628406b78c61d8796fd490922f17441576a5bfbe8501df")
        );
        assert_eq!(
            ExecutionRequestsGloas::default().hash_tree_root(&Sha2Hasher),
            hex!("87b69a306c8e430d0857f7c4ac5e27cecffa1108d43c2e5df7388056fea7a423")
        );
    }

    #[test]
    fn rejects_gloas_progressive_list_over_runtime_limit() {
        let mut request = gloas();
        let NewPayloadRequest::Gloas(request) = &mut request else {
            unreachable!();
        };
        request.execution_payload.withdrawals = vec![
            Withdrawal {
                index: 1,
                validator_index: 2,
                address: [3; 20],
                amount: 4,
            };
            MAX_WITHDRAWALS_PER_PAYLOAD + 1
        ]
        .into();

        assert!(matches!(
            NewPayloadRequest::Gloas(request.clone()).validate_progressive_limits(),
            Err(crate::guest::Error::ProgressiveListTooLong {
                field: "execution_payload.withdrawals",
                length,
                max: MAX_WITHDRAWALS_PER_PAYLOAD,
            }) if length == MAX_WITHDRAWALS_PER_PAYLOAD + 1
        ));
    }
}
