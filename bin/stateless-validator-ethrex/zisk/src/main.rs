//! ZisK Ethrex stateless validator guest program.

#![no_main]

use ere_platform_zisk::{ZiskPlatform, export_cycle_scope_names, ziskos};
use stateless_validator_ethrex::guest::{Guest, StatelessValidatorEthrexGuest};

ziskos::entrypoint!(main);

fn main() {
    StatelessValidatorEthrexGuest::run_output_sha256::<ZiskPlatform>();

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
