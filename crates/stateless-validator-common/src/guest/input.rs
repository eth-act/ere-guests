//! Canonical stateless validation input types.
//!
//! The types mirror [`stateless.py`] and their SSZ schemas in [`stateless_ssz.py`]. The wire
//! format is a 2-byte big-endian schema identifier followed by the SSZ-encoded `StatelessInput`
//! container.
//!
//! [`stateless.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.4.1/src/ethereum/forks/amsterdam/stateless.py
//! [`stateless_ssz.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.4.1/src/ethereum/forks/amsterdam/stateless_ssz.py

#![allow(missing_docs)]

use alloc::vec::Vec;
use core::{
    array,
    fmt::{self, Debug},
};

use libssz::{BYTES_PER_LENGTH_OFFSET, DecodeError, SszDecode, SszEncode};
use libssz_derive::{SszDecode, SszEncode};
use libssz_types::SszList;

use crate::guest::{error::Error, input::new_payload_request::NewPayloadRequest};

pub mod new_payload_request;

/// Schema identifier of the SSZ Amsterdam stateless input.
///
/// The spec fixes this schema to the Amsterdam payload shape. This
/// implementation extends it and accepts payload shape from Bellatrix onward
/// under the same identifier.
pub const STATELESS_INPUT_SCHEMA_ID: u16 = 0x0001;
/// Byte length of the big-endian schema identifier prefix.
pub const STATELESS_INPUT_SCHEMA_ID_SIZE: usize = 2;

/// SSZ list bounds from the Amsterdam stateless schema.
pub const MAX_WITNESS_NODES: usize = 1 << 20;
pub const MAX_WITNESS_CODES: usize = 1 << 16;
pub const MAX_WITNESS_HEADERS: usize = 256;
pub const MAX_BYTES_PER_WITNESS_NODE: usize = 1 << 20;
pub const MAX_BYTES_PER_CODE: usize = 1 << 24;
pub const MAX_BYTES_PER_HEADER: usize = 1 << 10;
pub const MAX_OPTIONAL_FORK_ACTIVATION_VALUES: usize = 1;
pub const MAX_BLOB_SCHEDULES_PER_FORK: usize = 1;
pub const MAX_PUBLIC_KEYS: usize = 1 << 20;
pub const PUBLIC_KEY_BYTES: usize = 65;

/// Execution witness data for stateless validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ExecutionWitness {
    /// Hashed trie-node preimages needed during execution and state-root recomputation.
    pub state: SszList<SszList<u8, MAX_BYTES_PER_WITNESS_NODE>, MAX_WITNESS_NODES>,
    /// Contract-code preimages (created or accessed) needed during execution.
    pub codes: SszList<SszList<u8, MAX_BYTES_PER_CODE>, MAX_WITNESS_CODES>,
    /// RLP-encoded block headers used for pre-state and `BLOCKHASH` correctness proofs. This may
    /// trend toward empty EIP-7709.
    pub headers: SszList<SszList<u8, MAX_BYTES_PER_HEADER>, MAX_WITNESS_HEADERS>,
}

/// Semantic execution-layer fork names understood by stateless inputs.
///
/// The discriminants are the SSZ enum values, which the spec derives from the declaration order
/// of `ProtocolFork`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u64)]
pub enum ProtocolFork {
    Frontier = 0,
    Homestead = 1,
    DAOFork = 2,
    TangerineWhistle = 3,
    SpuriousDragon = 4,
    Byzantium = 5,
    Constantinople = 6,
    ConstantinopleFix = 7,
    Istanbul = 8,
    MuirGlacier = 9,
    Berlin = 10,
    London = 11,
    ArrowGlacier = 12,
    GrayGlacier = 13,
    Paris = 14,
    Shanghai = 15,
    Cancun = 16,
    Prague = 17,
    Osaka = 18,
    BPO1 = 19,
    BPO2 = 20,
    BPO3 = 21,
    BPO4 = 22,
    BPO5 = 23,
    Amsterdam = 24,
}

impl ProtocolFork {
    /// Converts an SSZ enum value into a [`ProtocolFork`].
    pub fn from_u64(value: u64) -> Option<Self> {
        Some(match value {
            0 => Self::Frontier,
            1 => Self::Homestead,
            2 => Self::DAOFork,
            3 => Self::TangerineWhistle,
            4 => Self::SpuriousDragon,
            5 => Self::Byzantium,
            6 => Self::Constantinople,
            7 => Self::ConstantinopleFix,
            8 => Self::Istanbul,
            9 => Self::MuirGlacier,
            10 => Self::Berlin,
            11 => Self::London,
            12 => Self::ArrowGlacier,
            13 => Self::GrayGlacier,
            14 => Self::Paris,
            15 => Self::Shanghai,
            16 => Self::Cancun,
            17 => Self::Prague,
            18 => Self::Osaka,
            19 => Self::BPO1,
            20 => Self::BPO2,
            21 => Self::BPO3,
            22 => Self::BPO4,
            23 => Self::BPO5,
            24 => Self::Amsterdam,
            _ => return None,
        })
    }

    /// Returns the SSZ enum value of this fork.
    pub fn as_u64(self) -> u64 {
        self as u64
    }
}

impl SszEncode for ProtocolFork {
    fn is_fixed_size() -> bool {
        true
    }

    fn fixed_size() -> usize {
        <u64 as SszEncode>::fixed_size()
    }

    fn encoded_len(&self) -> usize {
        <u64 as SszEncode>::fixed_size()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        self.as_u64().ssz_append(buf);
    }
}

impl SszDecode for ProtocolFork {
    fn is_fixed_size() -> bool {
        true
    }

    fn fixed_size() -> usize {
        <u64 as SszDecode>::fixed_size()
    }

    /// Decodes a fork value, rejecting values outside the enumeration even
    /// though the spec schema models the field as a plain `uint64`. Unknown
    /// values are reported as an invalid union selector since the fork
    /// selects the [`NewPayloadRequest`] variant.
    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        let value = u64::from_ssz_bytes(bytes)?;
        Self::from_u64(value).ok_or(DecodeError::InvalidUnionSelector(
            u8::try_from(value).unwrap_or(u8::MAX),
        ))
    }
}

/// Activation point for a protocol fork.
///
/// The spec models both fields as optional values where at least one must be
/// set. The SSZ schema encodes each as a list holding zero or one element.
#[derive(Clone, Default, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ForkActivation {
    pub block_number: SszList<u64, MAX_OPTIONAL_FORK_ACTIVATION_VALUES>,
    pub timestamp: SszList<u64, MAX_OPTIONAL_FORK_ACTIVATION_VALUES>,
}

impl ForkActivation {
    /// Constructs a [`ForkActivation`] from optional activation values.
    pub fn new(block_number: Option<u64>, timestamp: Option<u64>) -> Self {
        let singleton = |value: Option<u64>| {
            SszList::try_from(value.into_iter().collect::<Vec<_>>())
                .expect("a list of at most one element is always within bounds")
        };
        Self {
            block_number: singleton(block_number),
            timestamp: singleton(timestamp),
        }
    }

    /// Returns the activation block number when present.
    pub fn block_number(&self) -> Option<u64> {
        self.block_number.first().copied()
    }

    /// Returns the activation timestamp when present.
    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp.first().copied()
    }

    /// Returns whether this activation point is active for a payload, applying the block-number
    /// and timestamp comparisons of `_is_activation_active` in the spec. The both-unset case, on
    /// which the spec raises, is rejected earlier by [`ChainConfig::validate`] and yields `false`
    /// here.
    pub fn is_active_at(&self, block_number: u64, timestamp: u64) -> bool {
        let activation_block_number = self.block_number();
        let activation_timestamp = self.timestamp();
        if activation_block_number.is_none() && activation_timestamp.is_none() {
            return false;
        }
        if activation_block_number.is_some_and(|at| block_number < at) {
            return false;
        }
        if activation_timestamp.is_some_and(|at| timestamp < at) {
            return false;
        }
        true
    }
}

impl Debug for ForkActivation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForkActivation")
            .field("block_number", &self.block_number.first())
            .field("timestamp", &self.timestamp.first())
            .finish()
    }
}

/// Effective blob parameters for a protocol fork.
#[derive(Debug, Clone, Copy, PartialEq, Eq, SszEncode, SszDecode)]
pub struct BlobSchedule {
    pub target: u64,
    pub max: u64,
    pub base_fee_update_fraction: u64,
}

/// Per-fork configuration needed to interpret stateless inputs.
#[derive(Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ForkConfig {
    pub fork: ProtocolFork,
    pub activation: ForkActivation,
    pub blob_schedule: SszList<BlobSchedule, MAX_BLOB_SCHEDULES_PER_FORK>,
}

impl ForkConfig {
    /// Constructs a [`ForkConfig`].
    pub fn new(
        fork: ProtocolFork,
        activation: ForkActivation,
        blob_schedule: Option<BlobSchedule>,
    ) -> Self {
        Self {
            fork,
            activation,
            blob_schedule: SszList::try_from(blob_schedule.into_iter().collect::<Vec<_>>())
                .expect("a list of at most one element is always within bounds"),
        }
    }

    /// Returns the blob schedule when present.
    pub fn blob_schedule(&self) -> Option<&BlobSchedule> {
        self.blob_schedule.first()
    }
}

impl Debug for ForkConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForkConfig")
            .field("fork", &self.fork)
            .field("activation", &self.activation)
            .field("blob_schedule", &self.blob_schedule.first())
            .finish()
    }
}

/// Chain configuration needed for stateless validation.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub active_fork: ForkConfig,
}

impl ChainConfig {
    /// Validates that the chain configuration is usable for the target payload, following
    /// `validate_chain_config` in the spec with two deliberate differences. The spec's
    /// blob-schedule equality check is performed by the verifier against the result public values
    /// instead of here, and the spec's `fork != Amsterdam` rejection is enforced when
    /// [`StatelessInput`] decoding selects the payload shape from the active fork.
    pub fn validate(&self, new_payload_request: &NewPayloadRequest) -> Result<(), Error> {
        if self.active_fork.activation.block_number().is_none()
            && self.active_fork.activation.timestamp().is_none()
        {
            return Err(Error::InvalidForkActivation);
        }

        if !self.active_fork.activation.is_active_at(
            new_payload_request.block_number(),
            new_payload_request.timestamp(),
        ) {
            return Err(Error::InactiveForkConfig);
        }

        Ok(())
    }
}

/// Canonical input to stateless validation.
///
/// Decoding selects the payload request shape from the active fork in `chain_config`. An input
/// whose payload does not match the fork fails to decode.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode)]
pub struct StatelessInput {
    /// Consensus-layer payload request to validate statelessly. See [`NewPayloadRequest`] for
    /// structure and links to consensus-specs.
    pub new_payload_request: NewPayloadRequest,
    /// Execution witness material required to re-execute the core state transition function
    /// statelessly.
    pub witness: ExecutionWitness,
    /// Chain configuration values needed during stateless validation.
    pub chain_config: ChainConfig,
    /// 65-byte uncompressed transaction public keys, in payload order.
    pub public_keys: SszList<[u8; PUBLIC_KEY_BYTES], MAX_PUBLIC_KEYS>,
}

impl StatelessInput {
    /// Serializes to schema-prefixed SSZ bytes, mirroring
    /// `serialize_stateless_input` in [`stateless_host.py`].
    ///
    /// [`stateless_host.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.4.1/src/ethereum/forks/amsterdam/stateless_host.py
    pub fn to_schema_prefixed_ssz(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(STATELESS_INPUT_SCHEMA_ID_SIZE + self.encoded_len());
        out.extend_from_slice(&STATELESS_INPUT_SCHEMA_ID.to_be_bytes());
        self.ssz_append(&mut out);
        out
    }

    /// Deserializes from schema-prefixed SSZ bytes, mirroring
    /// `deserialize_stateless_input` in [`stateless_guest.py`].
    ///
    /// [`stateless_guest.py`]: https://github.com/ethereum/execution-specs/blob/tests-zkevm@v0.4.1/src/ethereum/forks/amsterdam/stateless_guest.py
    pub fn from_schema_prefixed_ssz(bytes: &[u8]) -> Result<Self, Error> {
        let (schema_id, body) = bytes
            .split_first_chunk::<STATELESS_INPUT_SCHEMA_ID_SIZE>()
            .ok_or(Error::MissingSchemaId)?;
        let schema_id = u16::from_be_bytes(*schema_id);
        match schema_id {
            STATELESS_INPUT_SCHEMA_ID => Ok(Self::from_ssz_bytes(body)?),
            _ => Err(Error::UnsupportedSchemaId(schema_id)),
        }
    }
}

impl SszDecode for StatelessInput {
    fn is_fixed_size() -> bool {
        false
    }

    fn fixed_size() -> usize {
        0
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        use crate::guest::input::{NewPayloadRequest::*, ProtocolFork::*};

        let fields = ssz_split_var_fields::<4>(bytes)?;

        // Decode the other fields as usual.
        let witness = ExecutionWitness::from_ssz_bytes(fields[1])?;
        let chain_config = ChainConfig::from_ssz_bytes(fields[2])?;
        let public_keys = SszList::from_ssz_bytes(fields[3])?;

        // Decode `new_payload_request` from the shape the active fork selects.
        let new_payload_request = match chain_config.active_fork.fork {
            Paris => Bellatrix(SszDecode::from_ssz_bytes(fields[0])?),
            Shanghai => Capella(SszDecode::from_ssz_bytes(fields[0])?),
            Cancun => Deneb(SszDecode::from_ssz_bytes(fields[0])?),
            Prague | Osaka | BPO1 | BPO2 => ElectraFulu(SszDecode::from_ssz_bytes(fields[0])?),
            Amsterdam => Gloas(SszDecode::from_ssz_bytes(fields[0])?),
            fork => return Err(DecodeError::InvalidUnionSelector(fork.as_u64() as u8)),
        };

        Ok(Self {
            new_payload_request,
            witness,
            chain_config,
            public_keys,
        })
    }
}

/// Splits an SSZ container whose `N` fields are all variable-size.
fn ssz_split_var_fields<const N: usize>(bytes: &[u8]) -> Result<[&[u8]; N], DecodeError> {
    let fixed_part_len = N * BYTES_PER_LENGTH_OFFSET;
    if bytes.len() < fixed_part_len {
        return Err(DecodeError::InvalidByteLength {
            expected: fixed_part_len,
            got: bytes.len(),
        });
    }

    let mut offsets = [0usize; N];
    for i in 0..N {
        let offset =
            u32::from_le_bytes(array::from_fn(|j| bytes[i * BYTES_PER_LENGTH_OFFSET + j])) as usize;
        if i == 0 {
            if offset != fixed_part_len {
                return Err(DecodeError::InvalidFirstOffset {
                    expected: fixed_part_len,
                    got: offset,
                });
            }
        } else if offset < offsets[i - 1] {
            return Err(DecodeError::OffsetsAreNotMonotonicallyIncreasing);
        }
        if offset > bytes.len() {
            return Err(DecodeError::OffsetOutOfBounds {
                offset,
                length: bytes.len(),
            });
        }
        offsets[i] = offset;
    }

    Ok(array::from_fn(|i| {
        if i + 1 < N {
            &bytes[offsets[i]..offsets[i + 1]]
        } else {
            &bytes[offsets[i]..bytes.len()]
        }
    }))
}
