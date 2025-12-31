//! Airbender Reth stateless validator guest program.

#![no_std]
#![no_main]
#![no_builtins]
#![allow(incomplete_features)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]

use ere_platform_airbender::AirbenderPlatform;
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest};

mod airbender_rt;

fn main() {
    StatelessValidatorRethGuest::run_output_sha256::<AirbenderPlatform>();
}
