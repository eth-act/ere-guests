//! Helpers for compiling guest programs and asserting their zkVM execution.

use std::{collections::BTreeMap, fs, path::PathBuf, sync::LazyLock};

use dashmap::DashMap;
use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Elf, Input,
    ProverResource, zkVMKind,
};
use serde::Deserialize;

use crate::{
    execution::{ExecutionFailure, ExecutionOutput, GuestKind, run_execution},
    fixture::{FixturePreset, preset_fixtures},
};

/// Returns path to the workspace root.
fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

/// Resolves and caches guest ELF by compiling (ethrex, reth) or downloading (zesu).
pub fn resolve_guest(guest_kind: GuestKind, zkvm_kind: zkVMKind) -> Elf {
    static GUEST_ELF_CACHE: LazyLock<DashMap<(GuestKind, zkVMKind), Elf>> =
        LazyLock::new(DashMap::new);
    GUEST_ELF_CACHE
        .entry((guest_kind, zkvm_kind))
        .or_insert_with(|| match guest_kind {
            GuestKind::Ethrex | GuestKind::Reth => compile_guest(guest_kind, zkvm_kind),
            GuestKind::Zesu => download_guest(guest_kind, zkvm_kind),
        })
        .clone()
}

/// Compiles the guest program for `zkvm_kind` into an ELF.
pub fn compile_guest(guest_kind: GuestKind, zkvm_kind: zkVMKind) -> Elf {
    assert!(matches!(guest_kind, GuestKind::Ethrex | GuestKind::Reth));
    let workspace = workspace();
    let compiler =
        DockerizedCompiler::new(zkvm_kind, CompilerKind::RustCustomized, &workspace).unwrap();
    let dir = workspace
        .join("bin")
        .join(format!("stateless-validator-{}", guest_kind.as_str()))
        .join(zkvm_kind.as_str());
    compiler.compile(&dir, &[]).unwrap()
}

/// Downloads the prebuilt guest ELF listed in `artifact-registry.json`.
pub fn download_guest(guest_kind: GuestKind, zkvm_kind: zkVMKind) -> Elf {
    #[derive(Deserialize)]
    struct ArtifactRegistry {
        stateless_validator_elf: BTreeMap<String, GuestSoure>,
    }

    #[derive(Deserialize)]
    struct GuestSoure {
        url: String,
    }

    let registry = {
        let json = fs::read(workspace().join("artifact-registry.json")).unwrap();
        serde_json::from_slice::<ArtifactRegistry>(&json).unwrap()
    };
    let guest = format!("{}-{}", guest_kind.as_str(), zkvm_kind.as_str());
    let source = &registry
        .stateless_validator_elf
        .get(&guest)
        .unwrap_or_else(|| panic!("{guest} not found in artifact-registry.json"));
    Elf(reqwest::blocking::get(&source.url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec())
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

/// Builds the guest, then runs `preset`'s fixtures through it on `zkvm_kind`,
/// returning the failures.
pub fn run_stateless_validator_execution(
    guest_kind: GuestKind,
    zkvm_kind: zkVMKind,
    preset: FixturePreset,
) -> Vec<ExecutionFailure> {
    let elf = resolve_guest(guest_kind, zkvm_kind);
    let zkvm = init_zkvm(zkvm_kind, elf);
    run_execution(preset_fixtures(preset), &|input| {
        let output = zkvm.execute(&Input::new().with_stdin(input))?.0.to_vec();
        Ok(match guest_kind {
            GuestKind::Ethrex | GuestKind::Reth => ExecutionOutput::Hash(output),
            GuestKind::Zesu => ExecutionOutput::Bytes(output),
        })
    })
}
