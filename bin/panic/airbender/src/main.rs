//! Airbender panic guest program.

#![no_std]
#![no_main]
#![no_builtins]

use ere_platform_airbender::riscv_common::{csr_read_word, zksync_os_finish_success};

mod airbender_rt;

fn main() {
    if csr_read_word() == 0 {
        panic!("The ticker is eth");
    } else {
        zksync_os_finish_success(&[0; 8]);
    }
}
