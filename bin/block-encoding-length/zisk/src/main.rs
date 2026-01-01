//! ZisK block encoding length guest program.

#![no_main]

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_zisk::{ZiskPlatform, ziskos};

ziskos::entrypoint!(main);

fn main() {
    BlockEncodingLengthGuest::run::<ZiskPlatform>();
}
