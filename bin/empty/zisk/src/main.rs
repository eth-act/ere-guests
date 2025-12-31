//! ZisK emtpy guest program.

#![no_main]

use ere_platform_zisk::ziskos;

ziskos::entrypoint!(main);

fn main() {}
