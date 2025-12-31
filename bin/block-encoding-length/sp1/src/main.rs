//! SP1 block encoding length guest program.

#![no_main]

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_sp1::{SP1Platform, sp1_zkvm};

sp1_zkvm::entrypoint!(main);

fn main() {
    BlockEncodingLengthGuest::run::<SP1Platform>();
}
