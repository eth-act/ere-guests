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
            StatelessValidatorKind::Zesu => download_guest(stateless_validator_kind, zkvm_kind),
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
    compiler.compile(&dir, &[]).unwrap()
}

/// Downloads the prebuilt guest ELF listed in `artifact-registry.json`.
pub fn download_guest(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> Elf {
    #[derive(Deserialize)]
    struct ArtifactRegistry {
        stateless_validators: Vec<StatelessValidator>,
    }

    #[derive(Deserialize)]
    struct StatelessValidator {
        name: String,
        elfs: Vec<GuestElf>,
    }

    #[derive(Deserialize)]
    struct GuestElf {
        zkvm: String,
        url: String,
        sha256: String,
    }

    let registry = {
        let json = fs::read(workspace().join("artifact-registry.json")).unwrap();
        serde_json::from_slice::<ArtifactRegistry>(&json).unwrap()
    };
    let guest = format!(
        "{}-{}",
        stateless_validator_kind.as_str(),
        zkvm_kind.as_str()
    );
    let elf = registry
        .stateless_validators
        .iter()
        .find(|validator| validator.name == stateless_validator_kind.as_str())
        .and_then(|validator| {
            validator
                .elfs
                .iter()
                .find(|elf| elf.zkvm == zkvm_kind.as_str())
        })
        .unwrap_or_else(|| panic!("{guest} not found in artifact-registry.json"));
    let bytes = reqwest::blocking::get(&elf.url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();
    let sha256 = const_hex::encode(Sha256::digest(&bytes));
    assert_eq!(sha256, elf.sha256, "{guest} ELF sha256 mismatch");
    Elf(bytes)
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
