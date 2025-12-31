//! Pico Reth stateless validator guest program.

#![no_main]

use ere_platform_pico::{pico_sdk, PicoPlatform};
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest};

mod crypto;

pico_sdk::entrypoint!(main);

fn main() {
    crypto::install_crypto_provider();
    StatelessValidatorRethGuest::run_output_sha256::<PicoPlatform>();
}
