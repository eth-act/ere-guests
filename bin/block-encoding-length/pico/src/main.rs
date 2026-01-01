//! Pico block encoding length guest program.

#![no_main]

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_pico::{PicoPlatform, pico_sdk};

pico_sdk::entrypoint!(main);

fn main() {
    BlockEncodingLengthGuest::run::<PicoPlatform>();
}
