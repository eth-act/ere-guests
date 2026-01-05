//! Guest utilities.

#![no_std]

extern crate alloc;

mod guest;

pub use ere_io::Io;
pub use ere_platform_trait::Platform;
pub use guest::{Guest, GuestInput, GuestIo, GuestOutput};
