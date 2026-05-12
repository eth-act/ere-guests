//! Airbender panic guest program.

#![no_main]

use ere_platform_airbender::{airbender, entrypoint};

entrypoint!(main);

fn main() {
    if core::hint::black_box(false) {
        airbender::rt::sys::exit_success(&[0; 8]);
    }
    panic!("The ticker is eth")
}
