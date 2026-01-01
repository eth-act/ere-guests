//! Integration test lib.

use std::{ops::Deref, path::PathBuf};

use ere_dockerized::{CompilerKind, DockerizedCompiler, DockerizedzkVM, zkVMKind};
use ere_io::Io;
use ere_zkvm_interface::{Compiler, Input, ProverResourceType, zkVM};
use guest::{Guest, GuestInput, Platform};
use sha2::{Digest, Sha256};

/// Returns path to workspace
pub fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

/// Compiles guest program and initialize zkVM.
pub fn compile_and_init_zkvm(guest: &str, zkvm_kind: zkVMKind) -> DockerizedzkVM {
    let workspace = workspace();

    let compiler =
        DockerizedCompiler::new(zkvm_kind, CompilerKind::RustCustomized, &workspace).unwrap();
    let bin = workspace.join("bin").join(guest).join(zkvm_kind.as_str());
    let program = compiler.compile(&bin).unwrap();

    DockerizedzkVM::new(zkvm_kind, program, ProverResourceType::Cpu).unwrap()
}

/// Compiles guest program and runs execution, then check output are expected.
pub fn test_execution<G: Guest>(
    guest: &str,
    zkvm_kind: zkVMKind,
    inputs: impl IntoIterator<Item = GuestInput<G>>,
    is_output_sha256: bool,
) {
    let zkvm = compile_and_init_zkvm(guest, zkvm_kind);

    inputs.into_iter().for_each(|input| {
        let stdin = G::Io::serialize_input(&input).unwrap();
        let (public_values, _) = zkvm
            .execute(&Input::new().with_prefixed_stdin(stdin))
            .unwrap();

        let expected_public_values =
            G::Io::serialize_output(&G::compute::<HostPlatform>(input)).unwrap();

        // For those zkVMs that have fixed size output, treat all-zero as empty.
        if matches!(zkvm_kind, zkVMKind::Airbender | zkVMKind::OpenVM)
            && expected_public_values.is_empty()
        {
            assert!(public_values.into_iter().all(|byte| byte == 0));
        } else if is_output_sha256 {
            assert_eq!(public_values, *Sha256::digest(expected_public_values));
        } else {
            assert_eq!(public_values, expected_public_values);
        }
    });
}

struct HostPlatform;

impl Platform for HostPlatform {
    fn read_whole_input() -> impl Deref<Target = [u8]> {
        panic!("`Guest::compute` should not invoke `Platform::read_whole_input`");
        #[allow(unreachable_code)]
        Vec::new() // For `impl Deref<Target = [u8]>` to know the concrete type.
    }

    fn write_whole_output(_: &[u8]) {
        panic!("`Guest::compute` should not invoke `Platform::write_whole_output`")
    }

    fn print(message: &str) {
        print!("{message}");
    }
}
