//! ZisK Reth stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, export_cycle_scope_names, ziskos};
use stateless_validator_reth::guest::{Guest, StatelessValidatorRethGuest};

mod crypto;

ziskos::entrypoint!(main);

fn main() {
    // Install custom EVM crypto
    crypto::install_zisk_crypto().expect("failed to install ZisK revm crypto provider");

    StatelessValidatorRethGuest::run_output_sha256::<ZiskPlatform>();

    export_cycle_scope_names!(
        read_input,
        deserialize_input,
        new_payload_request_root_calculation,
        misc_preparation,
        new_payload_request_to_block,
        stf,
        serialize_output,
        sha256_output_bytes,
        write_output,
    );
}
