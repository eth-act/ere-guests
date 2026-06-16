//! Helpers for compiling guest programs and asserting their zkVM execution.

use std::{
    io::{self, Write},
    path::PathBuf,
};

use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Input,
    ProverResource, zkVMKind,
};
use ere_platform_core::Platform;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use stateless_validator_common::{
    HashTreeRoot, Sha2Hasher, SszDecode, SszEncode,
    guest::{StatelessInput, StatelessValidationResult},
};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::fixture::get_fixtures;

/// Returns path to the workspace root.
fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

/// Compiles the guest program and initializes the zkVM.
pub fn compile_and_init_zkvm(guest: &str, zkvm_kind: zkVMKind) -> DockerizedzkVM {
    let workspace = workspace();

    let compiler =
        DockerizedCompiler::new(zkvm_kind, CompilerKind::RustCustomized, &workspace).unwrap();
    let bin = workspace.join("bin").join(guest).join(zkvm_kind.as_str());
    let program = compiler.compile(&bin, &[]).unwrap();

    DockerizedzkVM::new(
        zkvm_kind,
        program,
        ProverResource::Cpu,
        DockerizedzkVMConfig::default(),
    )
    .unwrap()
}

/// Compiles the given stateless validator guest and asserts the zkVM
/// execution over the fixtures.
pub fn test_stateless_validator_execution(
    guest: &str,
    zkvm_kind: zkVMKind,
    execute_on_host: impl Fn(&[u8]) -> Vec<u8> + Sync,
) {
    let inputs = get_fixtures()
        .into_par_iter()
        .map(|fixture| {
            let output_bytes = if let Some(bytes) = &fixture.expected_output_bytes {
                bytes.clone()
            } else if fixture.success {
                let input = StatelessInput::from_schema_prefixed_ssz(&fixture.input_bytes).unwrap();
                let new_payload_request_root =
                    input.new_payload_request.hash_tree_root(&Sha2Hasher);
                StatelessValidationResult::new(new_payload_request_root, true, input.chain_config)
                    .to_ssz()
            } else {
                execute_on_host(&fixture.input_bytes)
            };
            let output = StatelessValidationResult::from_ssz_bytes(&output_bytes).unwrap();
            assert_eq!(output.successful_validation, fixture.success);

            TestCase::new(&fixture.name, fixture.input_bytes, output_bytes).output_sha256()
        })
        .collect::<Vec<_>>();
    test_execution(guest, zkvm_kind, inputs);
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

        if matches!(zkvm_kind, zkVMKind::Zisk) && expected_public_values.len() < 256 {
            expected_public_values.resize(256, 0);
        }

        assert_eq!(
            public_values.0, expected_public_values,
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
    /// Constructs a new [`TestCase`] from input bytes and expected output
    /// bytes.
    pub fn new(
        name: impl AsRef<str>,
        input_bytes: Vec<u8>,
        expected_public_values: Vec<u8>,
    ) -> Self {
        Self {
            name: name.as_ref().to_string(),
            input: Input::new().with_stdin(input_bytes),
            expected_public_values,
        }
    }

    /// Consumes the [`TestCase`] and constructs a new one with sha256 output.
    pub fn output_sha256(mut self) -> Self {
        self.expected_public_values = Sha256::digest(self.expected_public_values).to_vec();
        self
    }
}

/// A no-op platform for host-side guest execution that forwards debug messages to stdout.
#[derive(Debug)]
pub struct StdoutNoopPlatform;

impl Platform for StdoutNoopPlatform {
    #[allow(unreachable_code)]
    fn read_input() -> impl std::ops::Deref<Target = [u8]> {
        panic!("Can't read input in StdoutNoopPlatform");
        &[] as &[u8]
    }

    fn write_output(_: &[u8]) {
        panic!("Can't write output in StdoutNoopPlatform");
    }

    fn print(message: &str) {
        print!("{message}");
        let _ = io::stdout().flush();
    }
}
