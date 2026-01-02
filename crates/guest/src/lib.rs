//! Guest utilities.

#![no_std]

mod guest;

pub use ere_platform_trait::Platform;
pub use guest::{Guest, GuestInput, GuestIo, GuestOutput};
