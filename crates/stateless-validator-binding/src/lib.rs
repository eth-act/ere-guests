//! Beacon block binding prologue for stateless validator guests.
//!
//! The bare stateless validator commits the hash tree root of the payload request, which is only
//! computable from the payload, so a consensus client cannot verify the proof before downloading
//! it. This crate takes the beacon state fields that committed to the payload, rebuilds the beacon
//! block root from them, and derives the `StatelessInput` the execution engine validates. The
//! guest commits the [EIP-8025] public input, keyed on that beacon block root.
//!
//! [EIP-8025]: https://github.com/ethereum/consensus-specs/tree/master/specs/_features/eip8025

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod bind;
mod error;
pub mod input;
mod network;
pub mod types;
pub mod witness;

pub use bind::{BINDING_PROTOCOL_FORK, Bound, bind};
pub use error::Error;
pub use input::{BINDING_INPUT_SCHEMA_ID, BindingInput};
pub use network::{Mainnet, Network};
