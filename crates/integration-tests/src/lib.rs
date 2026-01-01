//! Integration test lib.

use std::path::PathBuf;

use ere_dockerized::{CompilerKind, DockerizedCompiler, zkVMKind};
use ere_zkvm_interface::Compiler;

/// Compiles guest program with zkVM, and runs execution and check public values
/// are expected.
pub fn test_execution(guest: &str, zkvm_kind: zkVMKind) {
    let workspace = workspace();

    let compiler =
        DockerizedCompiler::new(zkvm_kind, CompilerKind::RustCustomized, &workspace).unwrap();
    let bin = workspace.join("bin").join(guest).join(zkvm_kind.as_str());
    let _prorgam = compiler.compile(&bin).unwrap();

    // TODO: Run execution and check public values are expected.
}

fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}
