//! Helpers for compiling guest programs and asserting their zkVM execution.

use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use dashmap::DashMap;
use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Elf, Input,
    ProverResource, zkVMKind,
};
use serde::Deserialize;
use stateless_validator_common::guest::{
    StatelessInput, input::new_payload_request::NewPayloadRequest,
};

use crate::{
    execution::{ExecutionFailure, GuestKind, run_execution},
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
    static ELF: LazyLock<DashMap<(GuestKind, zkVMKind), Elf>> = LazyLock::new(DashMap::new);
    ELF.entry((guest_kind, zkvm_kind))
        .or_insert_with(|| match guest_kind {
            GuestKind::Ethrex | GuestKind::Reth => compile_guest(guest_kind, zkvm_kind),
            GuestKind::Zesu | GuestKind::Nethermind => download_guest(guest_kind, zkvm_kind),
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
        archive_elf_path: Option<String>,
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
    let bytes = reqwest::blocking::get(&source.url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();
    match &source.archive_elf_path {
        Some(archive_elf_path) => Elf(extract_elf_from_tar_gz(&bytes, archive_elf_path)),
        None => Elf(bytes),
    }
}

/// Extracts the entry at `archive_elf_path` from a gzip-compressed tar archive.
/// Some prebuilt guests, such as the nethermind ZisK release, distribute the ELF
/// inside a `.tar.gz` under an extensionless name recorded as `archive_elf_path`
/// in the registry.
fn extract_elf_from_tar_gz(bytes: &[u8], archive_elf_path: &str) -> Vec<u8> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.path().unwrap().as_ref() == Path::new(archive_elf_path) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).unwrap();
            return buf;
        }
    }
    panic!("{archive_elf_path} not found in archive")
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
        let input = match guest_kind {
            GuestKind::Nethermind => nethermind_input(input)?,
            _ => input,
        };
        Ok(zkvm.execute(&Input::new().with_stdin(input))?.0.to_vec())
    })
}

/// Rewrites the schema prefix of `ElectraFulu` variant payload to `0x0000`.
/// Reference: https://github.com/NethermindEth/nethermind/blob/zisk-guest-r7/src/Nethermind/Nethermind.Stateless.Executor/IO/InputDecoder.cs#L16-L23.
fn nethermind_input(stateless_input_bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let stateless_input = StatelessInput::from_schema_prefixed_ssz(&stateless_input_bytes)
        .map_err(|err| anyhow::anyhow!("decode stateless_input: {err:?}"))?;
    let schema_id = match &stateless_input.new_payload_request {
        NewPayloadRequest::ElectraFulu(_) => [0x00, 0x00],
        NewPayloadRequest::Gloas(_) => [0x00, 0x01],
        _ => anyhow::bail!("nethermind r7 supports only ElectraFulu (V3) and Gloas (V4) payloads"),
    };
    let mut stateless_input_bytes = stateless_input_bytes;
    stateless_input_bytes[0..2].copy_from_slice(&schema_id);
    Ok(stateless_input_bytes)
}
