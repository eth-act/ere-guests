//! Airbender block encoding length guest program.

#![no_std]
#![no_main]
#![no_builtins]
#![allow(incomplete_features)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]

use block_encoding_length::guest::{BlockEncodingLengthGuest, Guest};
use ere_platform_airbender::AirbenderPlatform;

mod airbender_rt;

fn main() {
    BlockEncodingLengthGuest::run::<AirbenderPlatform>();
}
