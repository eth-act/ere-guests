//! Risc0 Reth stateless validator guest program.

use ere_platform_risc0::Risc0Platform;
use stateless_validator_reth::guest::entrypoint;

fn main() {
    entrypoint::<Risc0Platform>();
}
