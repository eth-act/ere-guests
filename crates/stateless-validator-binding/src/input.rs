//! Wire format of the binding prologue input: a 2-byte big-endian schema identifier followed by
//! the SSZ-encoded container, as the stateless input does.

#![allow(missing_docs)]

use alloc::vec::Vec;

use libssz::{SszDecode, SszEncode};
use libssz_derive::{SszDecode, SszEncode};
use stateless_validator_common::guest::input::{ExecutionWitness, STATELESS_INPUT_SCHEMA_ID_SIZE};

use crate::{error::Error, types::ExecutionPayloadEnvelope, witness::BeaconStateWitness};

/// Schema identifier of the binding input: the Amsterdam fork index with revision `0x02`.
pub const BINDING_INPUT_SCHEMA_ID: u16 = 0x1502;

/// The prologue derives the `StatelessInput` from this, so the versioned hashes, the parent
/// beacon block root and the chain identifier are no longer supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq, SszEncode, SszDecode)]
pub struct BindingInput {
    pub state: BeaconStateWitness,
    pub envelope: ExecutionPayloadEnvelope,
    pub execution_witness: ExecutionWitness,
}

impl BindingInput {
    pub fn to_schema_prefixed_ssz(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(STATELESS_INPUT_SCHEMA_ID_SIZE + self.encoded_len());
        out.extend_from_slice(&BINDING_INPUT_SCHEMA_ID.to_be_bytes());
        self.ssz_append(&mut out);
        out
    }

    pub fn from_schema_prefixed_ssz(bytes: &[u8]) -> Result<Self, Error> {
        let (schema_id, body) = bytes
            .split_first_chunk::<STATELESS_INPUT_SCHEMA_ID_SIZE>()
            .ok_or(Error::MissingSchemaId)?;
        let schema_id = u16::from_be_bytes(*schema_id);
        if schema_id != BINDING_INPUT_SCHEMA_ID {
            return Err(Error::UnsupportedSchemaId(schema_id));
        }
        Ok(Self::from_ssz_bytes(body)?)
    }
}
