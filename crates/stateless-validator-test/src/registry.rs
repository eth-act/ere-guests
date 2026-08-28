//! Typed access to the release-backed guest artifact registry.

use std::collections::BTreeSet;

use anyhow::{Context, ensure};
use ere_dockerized::zkVMKind;
use serde::Deserialize;
use stateless_validator_catalog::StatelessValidatorKind;

const ARTIFACT_REGISTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../artifact-registry.json"
));

/// Release-backed stateless validator artifact registry.
#[derive(Clone, Debug, Deserialize)]
pub struct ArtifactRegistry {
    /// Active validators and their released artifacts.
    pub stateless_validators: Vec<StatelessValidator>,
}

/// One active stateless validator release.
#[derive(Clone, Debug, Deserialize)]
pub struct StatelessValidator {
    /// Catalog name.
    pub name: String,
    /// Upstream guest release version.
    pub version: String,
    /// Supported zkVM artifacts.
    pub artifacts: Vec<StatelessValidatorArtifact>,
}

/// One ELF/VK pair for a specific zkVM SDK.
#[derive(Clone, Debug, Deserialize)]
pub struct StatelessValidatorArtifact {
    /// Ere zkVM name.
    pub zkvm: String,
    /// Compatible zkVM SDK version.
    pub zkvm_version: String,
    /// Upstream ELF URL.
    pub elf_url: String,
    /// Expected ELF SHA-256.
    pub elf_sha256: String,
    /// Upstream program VK URL.
    pub vk_url: String,
    /// Expected program VK SHA-256.
    pub vk_sha256: String,
}

impl ArtifactRegistry {
    /// Parses and validates the workspace artifact registry.
    pub fn load() -> anyhow::Result<Self> {
        Self::parse(ARTIFACT_REGISTRY_JSON)
    }

    fn parse(json: &str) -> anyhow::Result<Self> {
        let registry: Self =
            serde_json::from_str(json).context("invalid artifact registry JSON")?;
        ensure!(
            !registry.stateless_validators.is_empty(),
            "artifact registry has no active stateless validators"
        );

        let mut validators = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        for validator in &registry.stateless_validators {
            validator
                .name
                .parse::<StatelessValidatorKind>()
                .with_context(|| {
                    format!("registry guest `{}` is not in the catalog", validator.name)
                })?;
            ensure!(
                validators.insert(validator.name.as_str()),
                "duplicate registry guest `{}`",
                validator.name
            );
            ensure!(
                !validator.version.is_empty(),
                "registry guest `{}` has no version",
                validator.name
            );
            ensure!(
                !validator.artifacts.is_empty(),
                "registry guest `{}` has no artifacts",
                validator.name
            );

            for artifact in &validator.artifacts {
                artifact.zkvm_kind().with_context(|| {
                    format!(
                        "registry guest `{}` has unsupported zkVM `{}`",
                        validator.name, artifact.zkvm
                    )
                })?;
                ensure!(
                    pairs.insert((validator.name.as_str(), artifact.zkvm.as_str())),
                    "duplicate registry artifact `{}-{}`",
                    validator.name,
                    artifact.zkvm
                );
                ensure!(
                    !artifact.zkvm_version.is_empty(),
                    "registry artifact `{}-{}` has no zkVM version",
                    validator.name,
                    artifact.zkvm
                );
                for (label, url, checksum) in [
                    ("ELF", &artifact.elf_url, &artifact.elf_sha256),
                    ("VK", &artifact.vk_url, &artifact.vk_sha256),
                ] {
                    ensure!(
                        url.starts_with("https://"),
                        "registry {label} URL for `{}-{}` is not HTTPS",
                        validator.name,
                        artifact.zkvm
                    );
                    ensure!(
                        checksum.len() == 64
                            && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
                        "registry {label} checksum for `{}-{}` is not SHA-256",
                        validator.name,
                        artifact.zkvm
                    );
                }
            }
        }
        Ok(registry)
    }

    /// Returns the artifact registered for `stateless_validator_kind` and `zkvm_kind`.
    pub fn artifact(
        &self,
        stateless_validator_kind: StatelessValidatorKind,
        zkvm_kind: zkVMKind,
    ) -> Option<&StatelessValidatorArtifact> {
        self.stateless_validators
            .iter()
            .find(|validator| validator.name == stateless_validator_kind.as_str())
            .and_then(|validator| {
                validator
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.zkvm == zkvm_kind.as_str())
            })
    }
}

impl StatelessValidatorArtifact {
    /// Returns the Ere zkVM kind named by this registry entry.
    pub fn zkvm_kind(&self) -> anyhow::Result<zkVMKind> {
        self.zkvm.parse().map_err(anyhow::Error::msg)
    }
}

#[cfg(test)]
mod tests {
    use ere_dockerized::zkVMKind;
    use stateless_validator_catalog::StatelessValidatorKind;

    use super::ArtifactRegistry;

    #[test]
    fn parses_active_registry() {
        let registry = ArtifactRegistry::load().unwrap();
        assert_eq!(registry.stateless_validators.len(), 2);

        for (kind, expected_version) in [
            (StatelessValidatorKind::Ethrex, "26.0.0-rc.2"),
            (StatelessValidatorKind::Reth, "0.1.0-rc.2"),
        ] {
            let validator = registry
                .stateless_validators
                .iter()
                .find(|validator| validator.name == kind.as_str())
                .unwrap();
            assert_eq!(kind.version(), Some(expected_version));
            assert_eq!(validator.version, expected_version);
            assert_eq!(validator.artifacts.len(), 3);

            for zkvm in [zkVMKind::OpenVM, zkVMKind::SP1, zkVMKind::Zisk] {
                assert!(registry.artifact(kind, zkvm).is_some());
            }
        }

        assert_eq!(StatelessValidatorKind::Zesu.version(), None);
    }

    #[test]
    fn rejects_duplicate_pairs() {
        let json = r#"{
            "stateless_validators": [{
                "name": "reth",
                "version": "test",
                "artifacts": [
                    {
                        "zkvm": "openvm",
                        "zkvm_version": "v1.0.0",
                        "elf_url": "https://example.com/a.elf",
                        "elf_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "vk_url": "https://example.com/a.vk",
                        "vk_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    },
                    {
                        "zkvm": "openvm",
                        "zkvm_version": "v1.0.0",
                        "elf_url": "https://example.com/b.elf",
                        "elf_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "vk_url": "https://example.com/b.vk",
                        "vk_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    }
                ]
            }]
        }"#;
        assert!(ArtifactRegistry::parse(json).is_err());
    }

    #[test]
    fn reports_missing_artifact() {
        let json = r#"{
            "stateless_validators": [{
                "name": "reth",
                "version": "test",
                "artifacts": [{
                    "zkvm": "openvm",
                    "zkvm_version": "v1.0.0",
                    "elf_url": "https://example.com/a.elf",
                    "elf_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "vk_url": "https://example.com/a.vk",
                    "vk_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                }]
            }]
        }"#;
        let registry = ArtifactRegistry::parse(json).unwrap();
        assert!(
            registry
                .artifact(StatelessValidatorKind::Reth, zkVMKind::SP1)
                .is_none()
        );
    }
}
