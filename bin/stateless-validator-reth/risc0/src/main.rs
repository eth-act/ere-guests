//! Risc0 Reth stateless validator guest program.

use ere_platform_risc0::Risc0Platform;
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest};

fn main() {
    StatelessValidatorRethGuest::run_output_sha256::<Risc0Platform>();
}
