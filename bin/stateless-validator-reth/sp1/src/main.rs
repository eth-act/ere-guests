//! SP1 Reth stateless validator guest program.

#![no_main]

use ere_platform_sp1::{SP1Platform, sp1_zkvm};
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest};

sp1_zkvm::entrypoint!(main);

fn main() {
    StatelessValidatorRethGuest::run_output_sha256::<SP1Platform>();
}
