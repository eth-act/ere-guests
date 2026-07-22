//! Generates the `StatelessValidatorKind::version` impl.

use std::{env, fs, path::Path};

use ere_util_build::{cargo_lock_path, detect_dep_version};

fn main() {
    let ethrex_version = detect_dep_version("stateless-validator-ethrex", "ethrex-guest-program");
    let reth_version = detect_dep_version("stateless-validator-reth", "reth-chainspec");

    let version_impl = format!(
        r#"impl crate::StatelessValidatorKind {{
    /// Returns the execution client version.
    pub const fn version(&self) -> &'static str {{
        match self {{
            Self::Ethrex => "{ethrex_version}",
            Self::Reth => "{reth_version}",
        }}
    }}
}}"#,
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let dst = Path::new(&out_dir).join("version_impl.rs");
    fs::write(dst, version_impl).unwrap();

    if let Some(cargo_lock) = cargo_lock_path() {
        println!("cargo:rerun-if-changed={}", cargo_lock.display());
    }
}
