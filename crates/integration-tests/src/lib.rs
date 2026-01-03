//! Integration test lib.

use std::{
    fs::{self, File},
    path::PathBuf,
};

use ere_dockerized::{CompilerKind, DockerizedCompiler, DockerizedzkVM, zkVMKind};
use ere_io::Io;
use ere_zkvm_interface::{Compiler, Input, ProverResourceType, zkVM};
use flate2::read::GzDecoder;
use guest::{Guest, GuestInput, GuestOutput};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use tar::Archive;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub mod stateless_validator;

/// Returns path to workspace
pub fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

/// Returns path to fixtures dir.
pub fn fixtures_dir() -> PathBuf {
    workspace().join("crates/integration-tests/fixtures")
}

/// Unpack all fixtures in fixtures dir.
pub fn untar_fixtures() -> std::io::Result<()> {
    let fixtures_dir = fixtures_dir();

    for entry in fs::read_dir(&fixtures_dir)? {
        let path = entry?.path();
        let filename = path.file_name().and_then(|filename| filename.to_str());
        if filename.is_some_and(|file_name| file_name.ends_with(".tar.gz")) {
            let file = File::open(&path)?;
            let gz = GzDecoder::new(file);
            Archive::new(gz).unpack(&fixtures_dir)?;
        }
    }

    Ok(())
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
pub fn test_execution(
    guest: &str,
    zkvm_kind: zkVMKind,
    test_cases: impl IntoIterator<Item = TestCase>,
) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let test_cases = test_cases.into_iter().collect::<Vec<_>>();
    assert!(!test_cases.is_empty());

    let zkvm = compile_and_init_zkvm(guest, zkvm_kind);

    test_cases.into_par_iter().for_each(|test_case| {
        info!("Running execution of test case {}", test_case.name);

        let (public_values, report) = zkvm.execute(&test_case.input).unwrap();

        info!(
            "Execution of test case {} took {:?}",
            test_case.name, report.execution_duration
        );

        let mut expected_public_values = test_case.expected_public_values;

        // Add padding for those zkVMs that have fixed size public values.
        if matches!(zkvm_kind, zkVMKind::Airbender | zkVMKind::OpenVM)
            && expected_public_values.len() < 32
        {
            expected_public_values.resize(32, 0);
        }

        assert_eq!(
            public_values, expected_public_values,
            "Expected public values of test case {} to be \
                {expected_public_values:?}, but got {public_values:?}",
            test_case.name
        );
    });
}

/// Guest program test case.
#[derive(Debug, Default)]
pub struct TestCase {
    /// Identifier of the test case.
    name: String,
    /// [`Input`] of the guest program.
    input: Input,
    /// The expected public values of guest program.
    expected_public_values: Vec<u8>,
}

impl TestCase {
    /// Constructs a new [`TestCase`].
    pub fn new<G: Guest>(
        name: impl AsRef<str>,
        input: GuestInput<G>,
        output: GuestOutput<G>,
    ) -> Self {
        Self {
            name: name.as_ref().to_string(),
            input: Input::new().with_prefixed_stdin(G::Io::serialize_input(&input).unwrap()),
            expected_public_values: G::Io::serialize_output(&output).unwrap(),
        }
    }

    /// Consumes the [`TestCase`] and constructs a new one with sha256 output.
    pub fn output_sha256(mut self) -> Self {
        self.expected_public_values = Sha256::digest(self.expected_public_values).to_vec();
        self
    }
}
