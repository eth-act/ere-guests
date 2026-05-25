//! Generate env `EL_VERSION`.

use ere_util_build::{cargo_lock_path, detect_dep_version};

fn main() {
    if let Some(cargo_lock) = cargo_lock_path() {
        println!("cargo:rerun-if-changed={}", cargo_lock.display());
    }

    let version = detect_dep_version("stateless-validator-ethrex", "ethrex-guest-program");
    println!("cargo:rustc-env=EL_VERSION={version}");
}
