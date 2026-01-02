//! Airbender empty guest program.

#![no_std]
#![no_main]
#![no_builtins]

use ere_platform_airbender::riscv_common::zksync_os_finish_success;

mod airbender_rt;

fn main() {
    zksync_os_finish_success(&[0; 8]);
}
