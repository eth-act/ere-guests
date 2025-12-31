//! Risc0 block encoding length guest program.

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_risc0::Risc0Platform;

fn main() {
    BlockEncodingLengthGuest::run::<Risc0Platform>();
}
