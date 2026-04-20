//! Guest utilities.

#![no_std]

extern crate alloc;

mod guest;

pub use ere_codec as codec;
pub use ere_platform_core::Platform;
pub use guest::{Guest, GuestInput, GuestOutput};
