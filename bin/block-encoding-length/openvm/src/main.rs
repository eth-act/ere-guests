//! OpenVM block encoding length guest program.

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_openvm::OpenVMPlatform;

fn main() {
    BlockEncodingLengthGuest::run::<OpenVMPlatform>();
}
