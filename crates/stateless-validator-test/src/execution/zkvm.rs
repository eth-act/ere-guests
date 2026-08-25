//! Helpers for resolving release-backed guests and executing them in zkVMs.

use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::ensure;
use dashmap::DashMap;
use ere_dockerized::{DockerizedzkVM, DockerizedzkVMConfig, Elf, Input, ProverResource, zkVMKind};
use semver::Version;
use sha2::{Digest, Sha256};
use stateless_validator_catalog::StatelessValidatorKind;

use crate::{
    execution::{ExecutionFailure, run_execution},
    fixture::StatelessValidatorFixture,
    registry::{ArtifactRegistry, StatelessValidatorArtifact},
};

/// Guest ELF paired with its published program VK.
#[derive(Clone, Debug)]
pub struct Guest {
    elf: Elf,
    vk: Vec<u8>,
}

/// Resolves and caches a guest exclusively from `artifact-registry.json`.
pub fn resolve_guest(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Guest {
    static GUEST: LazyLock<DashMap<(StatelessValidatorKind, zkVMKind), Guest>> =
        LazyLock::new(DashMap::new);

    GUEST
        .entry((stateless_validator_kind, zkvm_kind))
        .or_insert_with(|| download_guest(stateless_validator_kind, zkvm_kind))
        .clone()
}

/// Returns whether the registered guest ELF targets the zkVM SDK used by Ere.
pub fn is_guest_compatible(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> bool {
    matches_up_to_patch(
        &registry_artifact(stateless_validator_kind, zkvm_kind).zkvm_version,
        zkvm_kind.sdk_version(),
    )
}

/// Returns whether `version` and `other` are equal up to their patch level, comparing them
/// verbatim when either is not a `v`-prefixed semantic version.
fn matches_up_to_patch(version: &str, other: &str) -> bool {
    let parse = |version: &str| Version::parse(version.strip_prefix('v')?).ok();
    match (parse(version), parse(other)) {
        (Some(version), Some(other)) => {
            (version.major, version.minor, version.pre) == (other.major, other.minor, other.pre)
        }
        _ => version == other,
    }
}

fn registry_artifact(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> StatelessValidatorArtifact {
    ArtifactRegistry::load()
        .unwrap()
        .artifact(stateless_validator_kind, zkvm_kind)
        .unwrap_or_else(|| {
            panic!(
                "{}-{} not found in artifact-registry.json",
                stateless_validator_kind.as_str(),
                zkvm_kind.as_str()
            )
        })
        .clone()
}

fn verify_artifact_checksum(bytes: &[u8], sha256: &str) -> anyhow::Result<()> {
    ensure!(
        const_hex::encode(Sha256::digest(bytes)) == sha256,
        "artifact SHA-256 mismatch"
    );
    Ok(())
}

fn download_artifact(url: &str, sha256: &str) -> Vec<u8> {
    let bytes = reqwest::blocking::get(url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();
    verify_artifact_checksum(&bytes, sha256).unwrap_or_else(|error| panic!("{error} for {url}"));
    bytes
}

/// Downloads the registered ELF and program VK.
pub fn download_guest(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Guest {
    let artifact = registry_artifact(stateless_validator_kind, zkvm_kind);
    Guest {
        elf: Elf(download_artifact(&artifact.elf_url, &artifact.elf_sha256)),
        vk: download_artifact(&artifact.vk_url, &artifact.vk_sha256),
    }
}

/// Initializes and caches a CPU-backed zkVM for `elf`, reusing the one already running for
/// `zkvm_kind` unless it was initialized with a different ELF.
pub fn init_zkvm(zkvm_kind: zkVMKind, elf: Elf) -> Arc<DockerizedzkVM> {
    static ZKVM: LazyLock<DashMap<zkVMKind, Arc<DockerizedzkVM>>> = LazyLock::new(DashMap::new);

    drop(ZKVM.remove_if(&zkvm_kind, |_, zkvm| zkvm.elf() != &elf));

    let init = || {
        let config = DockerizedzkVMConfig {
            health_timeout: Duration::from_mins(15),
            ..Default::default()
        };
        Arc::new(DockerizedzkVM::new(zkvm_kind, elf, ProverResource::Cpu, config).unwrap())
    };
    ZKVM.entry(zkvm_kind).or_insert_with(init).clone()
}

/// Resolves the guest, verifies its published VK, and executes `fixtures` on `zkvm_kind`.
pub fn run_zkvm_execution(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
) -> Vec<ExecutionFailure> {
    let guest = resolve_guest(stateless_validator_kind, zkvm_kind);
    let zkvm = init_zkvm(zkvm_kind, guest.elf);
    assert_eq!(
        const_hex::encode_prefixed(zkvm.program_vk()),
        const_hex::encode_prefixed(&guest.vk),
        "regenerated program VK differs from the published one"
    );
    run_execution(fixtures, &|input| {
        Ok(zkvm.execute(&Input::new().with_stdin(input))?.0.to_vec())
    })
}

#[cfg(test)]
mod tests {
    use ere_dockerized::zkVMKind;
    use sha2::{Digest, Sha256};
    use stateless_validator_catalog::StatelessValidatorKind;

    use super::{matches_up_to_patch, verify_artifact_checksum};
    use crate::registry::ArtifactRegistry;

    #[test]
    fn compare_zkvm_versions() {
        for (version, other) in [
            ("v1.1.0-alpha", "v1.1.0-alpha"),
            ("v1.1.0-alpha", "v1.1.9-alpha"),
            ("v2.1.0", "v2.1.3"),
            ("8295d94", "8295d94"),
        ] {
            assert!(matches_up_to_patch(version, other));
        }

        for (version, other) in [
            ("v1.0.0-alpha", "v1.1.0-alpha"),
            ("v1.1.0-alpha", "v1.1.0-beta"),
            ("v1.1.0-alpha", "v1.1.0"),
            ("v1.1.0", "v2.1.0"),
            ("v1.1.0", "8295d94"),
            ("8295d94", "4df3d26"),
        ] {
            assert!(!matches_up_to_patch(version, other));
        }
    }

    #[test]
    fn reject_checksum_mismatch() {
        let checksum = const_hex::encode(Sha256::digest(b"expected"));
        verify_artifact_checksum(b"expected", &checksum).unwrap();
        assert!(verify_artifact_checksum(b"different", &checksum).is_err());
    }

    #[test]
    fn every_registered_artifact_matches_ere_sdk() {
        let registry = ArtifactRegistry::load().unwrap();
        for validator in &registry.stateless_validators {
            let kind = validator.name.parse::<StatelessValidatorKind>().unwrap();
            for artifact in &validator.artifacts {
                let zkvm = artifact.zkvm_kind().unwrap();
                assert!(
                    matches_up_to_patch(&artifact.zkvm_version, zkvm.sdk_version()),
                    "{kind}-{zkvm} targets {}, but Ere uses {}",
                    artifact.zkvm_version,
                    zkvm.sdk_version()
                );
            }
        }

        assert!(
            registry
                .artifact(StatelessValidatorKind::Reth, zkVMKind::OpenVM)
                .is_some()
        );
    }
}
