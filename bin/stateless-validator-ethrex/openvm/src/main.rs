//! OpenVM Ethrex stateless validator guest program.

use ere_platform_openvm::OpenVMPlatform;
use stateless_validator_ethrex::guest::entrypoint;

openvm::init!();

fn main() {
    entrypoint::<OpenVMPlatform>();
}
