//! Stateless Ethrex guest

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod guest;

/// Ethrex version.
pub const EL_VERSION: &str = env!("EL_VERSION");
