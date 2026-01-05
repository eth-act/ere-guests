//! Stateless validator common types and utilities.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod guest;

#[cfg(feature = "host")]
pub mod host;
