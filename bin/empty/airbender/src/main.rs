//! Airbender empty guest program.

#![no_main]

use ere_platform_airbender::{airbender, entrypoint};

entrypoint!(main);

fn main() {
    airbender::rt::sys::exit_success(&[0; 8]);
}
