//! Pico panic guest program.

#![no_main]

use ere_platform_pico::pico_sdk;

pico_sdk::entrypoint!(main);

fn main() {
    panic!("The ticker is eth")
}
