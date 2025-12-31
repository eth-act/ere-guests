use ere_platform_risc0::Risc0Platform;
use stateless_validator_ethrex::guest::{Guest, StatelessValidatorEthrexGuest};

pub fn main() {
    StatelessValidatorEthrexGuest::run_output_sha256::<Risc0Platform>();
}
