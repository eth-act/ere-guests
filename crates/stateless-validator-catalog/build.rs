//! Generates the `StatelessValidatorKind::version` impl.

use std::{env, fs, path::Path};

use ere_util_build::{cargo_lock_path, detect_dep_version, workspace};
use serde::Deserialize;

fn main() {
    let ethrex_version = detect_dep_version("stateless-validator-ethrex", "ethrex-guest-program");
    let reth_version = detect_dep_version("stateless-validator-reth", "reth-chainspec");
    let zesu_version = registry_version("zesu");

    let version_impl = format!(
        r#"impl crate::StatelessValidatorKind {{
    /// Returns the stateless validator version.
    pub const fn version(&self) -> &'static str {{
        match self {{
            Self::Ethrex => "{ethrex_version}",
            Self::Reth => "{reth_version}",
            Self::Zesu => "{zesu_version}",
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

/// Resolves the version of the `name` stateless validator from `artifact-registry.json`.
fn registry_version(name: &str) -> String {
    #[derive(Deserialize)]
    struct ArtifactRegistry {
        stateless_validators: Vec<StatelessValidator>,
    }

    #[derive(Deserialize)]
    struct StatelessValidator {
        name: String,
        version: String,
    }

    let registry_path = workspace()
        .expect("workspace should be found")
        .join("artifact-registry.json");
    println!("cargo:rerun-if-changed={}", registry_path.display());

    let registry =
        serde_json::from_slice::<ArtifactRegistry>(&fs::read(&registry_path).unwrap()).unwrap();
    registry
        .stateless_validators
        .into_iter()
        .find(|validator| validator.name == name)
        .unwrap_or_else(|| panic!("`{name}` not found in artifact-registry.json"))
        .version
}
