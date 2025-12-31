//! ZisK panic guest program.

#![no_main]

use ere_platform_zisk::ziskos;

ziskos::entrypoint!(main);

fn main() {
    panic!("The ticker is eth")
}
