//! Helpers for compiling guest programs and asserting their zkVM execution.

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::Duration,
};

use dashmap::DashMap;
use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Elf, Input,
    ProverResource, zkVMKind,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use stateless_validator_catalog::StatelessValidatorKind::{self, *};

use crate::{
    execution::{ExecutionFailure, run_execution},
    fixture::StatelessValidatorFixture,
};

/// Returns path to the workspace root.
fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

/// Guest ELF paired with the published program VK when the guest is a republished one.
#[derive(Clone, Debug)]
pub struct Guest {
    elf: Elf,
    vk: Option<Vec<u8>>,
}

/// Resolves and caches guest artifacts by compiling (ethrex, reth) or downloading (zesu).
pub fn resolve_guest(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Guest {
    static GUEST: LazyLock<DashMap<(StatelessValidatorKind, zkVMKind), Guest>> =
        LazyLock::new(DashMap::new);

    let resolve = || match stateless_validator_kind {
        Ethrex | Reth => compile_guest(stateless_validator_kind, zkvm_kind),
        Zesu => download_guest(stateless_validator_kind, zkvm_kind),
    };
    GUEST
        .entry((stateless_validator_kind, zkvm_kind))
        .or_insert_with(resolve)
        .clone()
}

/// Compiles the guest program for `zkvm_kind` into an ELF.
pub fn compile_guest(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Guest {
    assert!(matches!(stateless_validator_kind, Ethrex | Reth));
    let workspace = workspace();
    let compiler =
        DockerizedCompiler::new(zkvm_kind, CompilerKind::RustCustomized, &workspace).unwrap();
    let dir = workspace
        .join("bin")
        .join(format!(
            "stateless-validator-{}",
            stateless_validator_kind.as_str()
        ))
        .join(zkvm_kind.as_str());
    let options = match stateless_validator_kind {
        Ethrex => vec![],
        Reth => vec!["--ignore-rust-version".to_string()],
        _ => unreachable!(),
    };
    let elf = compiler.compile(&dir, &options).unwrap();
    Guest { elf, vk: None }
}

/// Wire shape of `artifact-registry.json`.
#[derive(Deserialize)]
struct ArtifactRegistry {
    stateless_validators: Vec<StatelessValidator>,
}

#[derive(Deserialize)]
struct StatelessValidator {
    name: String,
    artifacts: Vec<StatelessValidatorArtifact>,
}

#[derive(Deserialize)]
struct StatelessValidatorArtifact {
    zkvm: String,
    elf_url: String,
    elf_sha256: String,
    vk_url: Option<String>,
    vk_sha256: Option<String>,
}

/// Returns the `artifact-registry.json` entry of the guest.
fn registry_artifact(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> StatelessValidatorArtifact {
    let json = fs::read(workspace().join("artifact-registry.json")).unwrap();
    serde_json::from_slice::<ArtifactRegistry>(&json)
        .unwrap()
        .stateless_validators
        .into_iter()
        .find(|validator| validator.name == stateless_validator_kind.as_str())
        .and_then(|validator| {
            validator
                .artifacts
                .into_iter()
                .find(|artifact| artifact.zkvm == zkvm_kind.as_str())
        })
        .unwrap_or_else(|| {
            panic!(
                "{}-{} not found in artifact-registry.json",
                stateless_validator_kind.as_str(),
                zkvm_kind.as_str()
            )
        })
}

/// Downloads the artifact at `url`, asserting its sha256 matches `sha256`.
fn download_artifact(url: &str, sha256: &str) -> Vec<u8> {
    let bytes = reqwest::blocking::get(url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();
    assert_eq!(
        const_hex::encode(Sha256::digest(&bytes)),
        sha256,
        "sha256 mismatch of {url}"
    );
    bytes
}

/// Downloads the ELF and the optional VK listed in `artifact-registry.json`.
pub fn download_guest(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Guest {
    let artifact = registry_artifact(stateless_validator_kind, zkvm_kind);
    let elf = Elf(download_artifact(&artifact.elf_url, &artifact.elf_sha256));
    let vk = artifact
        .vk_url
        .zip(artifact.vk_sha256)
        .map(|(url, sha256)| download_artifact(&url, &sha256));
    Guest { elf, vk }
}

/// Initializes and caches a CPU-backed zkVM for `elf`, reusing the one already running for
/// `zkvm_kind` unless it was initialized with a different ELF.
pub fn init_zkvm(zkvm_kind: zkVMKind, elf: Elf) -> Arc<DockerizedzkVM> {
    static ZKVM: LazyLock<DashMap<zkVMKind, Arc<DockerizedzkVM>>> = LazyLock::new(DashMap::new);

    // Shuts down the stale zkVM before initializing the new one.
    drop(ZKVM.remove_if(&zkvm_kind, |_, zkvm| zkvm.elf() != &elf));

    let init = || {
        let resource = ProverResource::Cpu;
        let config = DockerizedzkVMConfig {
            health_timeout: Duration::from_mins(15),
            ..Default::default()
        };
        Arc::new(DockerizedzkVM::new(zkvm_kind, elf, resource, config).unwrap())
    };
    ZKVM.entry(zkvm_kind).or_insert_with(init).clone()
}

/// Resolves the guest ELF, asserts the published VK can be regenerated by Ere when the guest lists
/// one, then runs `fixtures` through it on `zkvm_kind`, returning the failures.
pub fn run_zkvm_execution(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
) -> Vec<ExecutionFailure> {
    let guest = resolve_guest(stateless_validator_kind, zkvm_kind);
    let zkvm = init_zkvm(zkvm_kind, guest.elf);
    if let Some(vk) = guest.vk {
        assert_eq!(
            const_hex::encode_prefixed(zkvm.program_vk()),
            const_hex::encode_prefixed(&vk),
            "regenerated program VK differs from the published one"
        );
    }
    run_execution(fixtures, &|input| {
        Ok(zkvm.execute(&Input::new().with_stdin(input))?.0.to_vec())
    })
}
