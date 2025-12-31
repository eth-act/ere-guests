//! Pico block encoding length guest program.

#![no_main]

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_pico::{pico_sdk, PicoPlatform};

pico_sdk::entrypoint!(main);

fn main() {
    BlockEncodingLengthGuest::run::<PicoPlatform>();
}
