//! Generates the `StatelessValidatorKind::version` impl from the artifact registry.

use std::{collections::BTreeSet, env, fs, path::Path};

use serde::Deserialize;

fn main() {
    let registry_path = workspace().join("artifact-registry.json");
    println!("cargo:rerun-if-changed={}", registry_path.display());
    let registry = serde_json::from_slice::<ArtifactRegistry>(&fs::read(&registry_path).unwrap())
        .expect("artifact-registry.json should be valid");

    let mut names = BTreeSet::new();
    let arms = registry
        .stateless_validators
        .into_iter()
        .map(|validator| {
            assert!(
                names.insert(validator.name.clone()),
                "duplicate stateless validator `{}` in artifact-registry.json",
                validator.name
            );
            let variant = variant_name(&validator.name);
            format!(
                "            Self::{variant} => Some({:?}),",
                validator.version
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let version_impl = format!(
        r#"impl crate::StatelessValidatorKind {{
    /// Returns the active stateless validator version, or `None` when no artifacts are registered.
    pub const fn version(&self) -> Option<&'static str> {{
        #[allow(unreachable_patterns)]
        match self {{
{arms}
            _ => None,
        }}
    }}
}}"#,
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let dst = Path::new(&out_dir).join("version_impl.rs");
    fs::write(dst, version_impl).unwrap();
}

#[derive(Deserialize)]
struct ArtifactRegistry {
    stateless_validators: Vec<StatelessValidator>,
}

#[derive(Deserialize)]
struct StatelessValidator {
    name: String,
    version: String,
}

fn workspace() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("catalog crate should be under the workspace crates directory")
        .to_path_buf()
}

fn variant_name(name: &str) -> String {
    name.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .into_iter()
                .flat_map(char::to_uppercase)
                .chain(chars)
                .collect::<String>()
        })
        .collect()
}
