//! Helpers for compiling guest programs and asserting their zkVM execution.

use std::{fs, path::PathBuf};

use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Elf, Input,
    ProverResource, zkVMKind,
};

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
    assert!(matches!(guest_kind, GuestKind::Zesu));
    let artifact_registry: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace().join("artifact-registry.json")).unwrap())
            .unwrap();
    let guest = format!("{}-{}", guest_kind.as_str(), zkvm_kind.as_str());
    let url: String =
        serde_json::from_value(artifact_registry["stateless_validator_elf"][guest]["url"].clone())
            .unwrap();
    Elf(reqwest::blocking::get(url)
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
    let elf = match guest_kind {
        GuestKind::Ethrex | GuestKind::Reth => compile_guest(guest_kind, zkvm_kind),
        GuestKind::Zesu => download_guest(guest_kind, zkvm_kind),
    };
    let zkvm = init_zkvm(zkvm_kind, elf);
    run_execution(preset_fixtures(preset), &|input| {
        let output = zkvm.execute(&Input::new().with_stdin(input))?.0.to_vec();
        Ok(match guest_kind {
            GuestKind::Ethrex | GuestKind::Reth => ExecutionOutput::Hash(output),
            GuestKind::Zesu => ExecutionOutput::Bytes(output),
        })
    })
}
