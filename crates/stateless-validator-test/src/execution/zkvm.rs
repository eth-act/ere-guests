//! Helpers for compiling guest programs and asserting their zkVM execution.

use std::{fs, path::PathBuf, sync::LazyLock};

use dashmap::DashMap;
use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Elf, Input,
    ProverResource, zkVMKind,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use stateless_validator_catalog::StatelessValidatorKind;

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

/// Resolves and caches guest ELF by compiling (ethrex, reth) or downloading (zesu).
pub fn resolve_guest(stateless_validator_kind: StatelessValidatorKind, zkvm_kind: zkVMKind) -> Elf {
    static ELF: LazyLock<DashMap<(StatelessValidatorKind, zkVMKind), Elf>> =
        LazyLock::new(DashMap::new);
    ELF.entry((stateless_validator_kind, zkvm_kind))
        .or_insert_with(|| match stateless_validator_kind {
            StatelessValidatorKind::Ethrex | StatelessValidatorKind::Reth => {
                compile_guest(stateless_validator_kind, zkvm_kind)
            }
            StatelessValidatorKind::Zesu => download_elf(stateless_validator_kind, zkvm_kind),
        })
        .clone()
}

/// Compiles the guest program for `zkvm_kind` into an ELF.
pub fn compile_guest(stateless_validator_kind: StatelessValidatorKind, zkvm_kind: zkVMKind) -> Elf {
    assert!(matches!(
        stateless_validator_kind,
        StatelessValidatorKind::Ethrex | StatelessValidatorKind::Reth
    ));
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
        StatelessValidatorKind::Ethrex => vec![],
        StatelessValidatorKind::Reth => vec!["--ignore-rust-version".to_string()],
        _ => unreachable!(),
    };
    compiler.compile(&dir, &options).unwrap()
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

/// Downloads the ELF listed in `artifact-registry.json`.
pub fn download_elf(stateless_validator_kind: StatelessValidatorKind, zkvm_kind: zkVMKind) -> Elf {
    let artifact = registry_artifact(stateless_validator_kind, zkvm_kind);
    Elf(download_artifact(&artifact.elf_url, &artifact.elf_sha256))
}

/// Downloads the VK listed in `artifact-registry.json`.
pub fn download_vk(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Vec<u8> {
    let artifact = registry_artifact(stateless_validator_kind, zkvm_kind);
    let (url, sha256) = artifact.vk_url.zip(artifact.vk_sha256).unwrap_or_else(|| {
        panic!(
            "{}-{} lists no VK in artifact-registry.json",
            stateless_validator_kind.as_str(),
            zkvm_kind.as_str()
        )
    });
    download_artifact(&url, &sha256)
}

/// Initializes a CPU-backed zkVM for `elf`.
pub fn init_zkvm(zkvm_kind: zkVMKind, elf: Elf) -> DockerizedzkVM {
    DockerizedzkVM::new(
        zkvm_kind,
        elf,
        ProverResource::Cpu,
        DockerizedzkVMConfig::default(),
    )
    .unwrap()
}

/// Resolves the guest ELF, then runs `fixtures` through it on `zkvm_kind`,
/// returning the failures.
pub fn run_zkvm_execution(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
    fixtures: impl IntoIterator<Item = StatelessValidatorFixture>,
) -> Vec<ExecutionFailure> {
    let elf = resolve_guest(stateless_validator_kind, zkvm_kind);
    let zkvm = init_zkvm(zkvm_kind, elf);
    run_execution(fixtures, &|input| {
        Ok(zkvm.execute(&Input::new().with_stdin(input))?.0.to_vec())
    })
}
