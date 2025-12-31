//! Risc0 Ethrex stateless validator guest program.

use ere_platform_risc0::Risc0Platform;
use stateless_validator_ethrex::guest::{Guest, StatelessValidatorEthrexGuest};

fn main() {
    StatelessValidatorEthrexGuest::run_output_sha256::<Risc0Platform>();
}
