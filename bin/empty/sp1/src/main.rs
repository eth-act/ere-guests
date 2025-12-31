//! SP1 emtpy guest program.

#![no_main]

use ere_platform_sp1::sp1_zkvm;

sp1_zkvm::entrypoint!(main);

fn main() {}
