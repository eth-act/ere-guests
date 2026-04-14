//! Host-side debug runner for stateless validator guest fixtures.

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use clap::{Parser, ValueEnum};
use flate2::read::GzDecoder;
use guest::{Guest, Platform};
use serde::Deserialize;
use stateless::StatelessInput;
use stateless_validator_ethrex::guest::{
    StatelessValidatorEthrexGuest, StatelessValidatorEthrexInput,
};
use stateless_validator_reth::guest::{
    StatelessValidatorOutput, StatelessValidatorRethGuest, StatelessValidatorRethInput,
};
use tar::Archive;
use tempfile::TempDir;
use tracing_subscriber::EnvFilter;

/// CLI options for the stateless validator debug runner.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "stateless-validator-debug",
    about = "Run stateless validator guest fixtures directly on the host.",
    long_about = None,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Guest program to run.
    #[arg(long, value_enum)]
    pub guest: GuestKind,
    /// Output format for each fixture result.
    #[arg(long = "format", value_enum, default_value_t = OutputFormat::Summary)]
    pub output_format: OutputFormat,
    /// Warn and continue when fixture success does not match guest output.
    #[arg(long)]
    pub allow_success_mismatch: bool,
    /// Path to a fixture JSON file, `.tar.gz` archive, or directory.
    pub path: PathBuf,
}

/// Collected fixture paths plus any temporary extraction directory they depend on.
#[derive(Debug)]
struct PreparedFixturePaths {
    /// The concrete JSON fixture files to execute.
    paths: Vec<PathBuf>,
    /// Temporary directory holding extracted fixtures from an archive input.
    _extracted_dir: Option<TempDir>,
}

/// Stateless validator guest selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GuestKind {
    /// Run the Reth stateless validator guest.
    Reth,
    /// Run the Ethrex stateless validator guest.
    Ethrex,
}

impl GuestKind {
    fn run_fixture(self, fixture: &StatelessValidatorFixture) -> anyhow::Result<RunSummary> {
        let block_hash = fixture.stateless_input.block.hash_slow().0;
        let output: StatelessValidatorOutput = match self {
            Self::Reth => {
                let input =
                    StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success)?;
                StatelessValidatorRethGuest::compute::<StdoutNoopPlatform>(input)
            }
            Self::Ethrex => {
                let input =
                    StatelessValidatorEthrexInput::new(&fixture.stateless_input, fixture.success)?;
                StatelessValidatorEthrexGuest::compute::<StdoutNoopPlatform>(input)
            }
        };

        Ok(RunSummary {
            fixture_name: fixture.name.clone(),
            guest: self,
            expected_success: fixture.success,
            actual_success: output.successful_block_validation,
            block_hash,
            new_payload_request_root: output.new_payload_request_root,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Reth => "reth",
            Self::Ethrex => "ethrex",
        }
    }
}

/// Output format for fixture execution summaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Emit a human-readable summary line for each fixture.
    Summary,
    /// Emit copy-pasteable Rust map entries keyed by block hash.
    RustMap,
}

/// Deserialized JSON fixture supported by the debug runner.
#[derive(Debug, Clone, Deserialize)]
pub struct StatelessValidatorFixture {
    /// Human-readable fixture identifier.
    pub name: String,
    /// Stateless input consumed by the host-side input builders.
    pub stateless_input: StatelessInput,
    /// Expected validation outcome.
    pub success: bool,
}

/// Summary of one guest execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    /// Name of the fixture that ran.
    pub fixture_name: String,
    /// Guest program that was executed.
    pub guest: GuestKind,
    /// Expected guest success from the fixture.
    pub expected_success: bool,
    /// Actual guest success reported by the guest output.
    pub actual_success: bool,
    /// Canonical execution block hash for the fixture.
    pub block_hash: [u8; 32],
    /// The resulting new payload request root.
    pub new_payload_request_root: [u8; 32],
}

impl std::fmt::Display for RunSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fixture={} guest={} expected_success={} actual_success={} new_payload_request_root=0x{}",
            self.fixture_name,
            self.guest.as_str(),
            self.expected_success,
            self.actual_success,
            encode_hex(&self.new_payload_request_root)
        )?;
        write!(f, " block_hash=0x{}", encode_hex(&self.block_hash))
    }
}

impl RunSummary {
    fn rust_map_entry(&self) -> String {
        format!(
            "(b256!(\"{}\"), b256!(\"{}\")), // {}",
            encode_hex(&self.block_hash),
            encode_hex(&self.new_payload_request_root),
            self.fixture_name,
        )
    }
}

/// A no-op platform for host-side guest execution that forwards debug messages to stdout.
#[derive(Debug)]
pub struct StdoutNoopPlatform;

impl Platform for StdoutNoopPlatform {
    #[allow(unreachable_code)]
    fn read_whole_input() -> impl std::ops::Deref<Target = [u8]> {
        panic!("Can't read input in StdoutNoopPlatform");
        &[] as &[u8]
    }

    fn write_whole_output(_: &[u8]) {
        panic!("Can't write output in StdoutNoopPlatform");
    }

    fn print(message: &str) {
        println!("{message}");
        let _ = io::stdout().flush();
    }
}

/// Entry point for the debug runner binary.
pub fn main_entry() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let output_format = cli.output_format;
    execute(cli, |summary| match output_format {
        OutputFormat::Summary => println!("{summary}"),
        OutputFormat::RustMap => println!("{}", summary.rust_map_entry()),
    })
}

/// Executes one or more fixtures and reports each summary via `on_summary`.
pub fn execute(cli: Cli, mut on_summary: impl FnMut(&RunSummary)) -> anyhow::Result<()> {
    let fixture_paths = prepare_fixture_paths(&cli.path)?;

    for fixture_path in &fixture_paths.paths {
        let fixture = load_fixture(fixture_path)?;
        let summary = cli
            .guest
            .run_fixture(&fixture)
            .with_context(|| format!("failed to execute fixture {}", fixture_path.display()))?;
        on_summary(&summary);

        handle_success_mismatch(&summary, fixture_path, cli.allow_success_mismatch)?;
    }

    Ok(())
}

fn prepare_fixture_paths(path: &Path) -> anyhow::Result<PreparedFixturePaths> {
    if path.is_file() && is_tar_gz_path(path) {
        let extracted_dir = tempfile::tempdir()
            .context("failed to create temporary directory for fixture archive")?;
        unpack_fixture_archive(path, extracted_dir.path())?;
        let paths = collect_fixture_paths_recursive(extracted_dir.path()).with_context(|| {
            format!("failed to collect fixtures from archive {}", path.display())
        })?;

        return Ok(PreparedFixturePaths {
            paths,
            _extracted_dir: Some(extracted_dir),
        });
    }

    Ok(PreparedFixturePaths {
        paths: collect_fixture_paths(path)?,
        _extracted_dir: None,
    })
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .try_init();
}

fn handle_success_mismatch(
    summary: &RunSummary,
    fixture_path: &Path,
    allow_success_mismatch: bool,
) -> anyhow::Result<()> {
    if summary.actual_success == summary.expected_success {
        return Ok(());
    }

    if allow_success_mismatch {
        tracing::warn!(
            fixture_name = summary.fixture_name.as_str(),
            fixture_path = %fixture_path.display(),
            expected_success = summary.expected_success,
            actual_success = summary.actual_success,
            "fixture success mismatch",
        );
        return Ok(());
    }

    bail!(
        "fixture {} ({}) expected success={}, got success={}",
        summary.fixture_name,
        fixture_path.display(),
        summary.expected_success,
        summary.actual_success,
    );
}

/// Collects fixture file paths from a JSON file or a directory.
pub fn collect_fixture_paths(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            bail!(
                "fixture file {} must have a .json extension",
                path.display()
            );
        }
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.exists() {
        bail!("path {} does not exist", path.display());
    }

    if !path.is_dir() {
        bail!("path {} must be a file or directory", path.display());
    }

    let mut paths = fs::read_dir(path)
        .with_context(|| format!("failed to read fixture directory {}", path.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let entry_path = entry.path();
            let file_type = entry.file_type().ok()?;
            (file_type.is_file()
                && entry_path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .then_some(entry_path)
        })
        .collect::<Vec<_>>();
    paths.sort();

    if paths.is_empty() {
        bail!("no JSON fixtures found in {}", path.display());
    }

    Ok(paths)
}

fn collect_fixture_paths_recursive(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut directories = vec![path.to_path_buf()];
    let mut paths = Vec::new();

    while let Some(directory) = directories.pop() {
        let entries = fs::read_dir(&directory)
            .with_context(|| format!("failed to read fixture directory {}", directory.display()))?;

        for entry in entries {
            let entry = entry.with_context(|| {
                format!(
                    "failed to read fixture directory entry in {}",
                    directory.display()
                )
            })?;
            let entry_path = entry.path();
            let file_type = entry.file_type().with_context(|| {
                format!("failed to inspect fixture path {}", entry_path.display())
            })?;

            if file_type.is_dir() {
                directories.push(entry_path);
                continue;
            }

            if file_type.is_file()
                && entry_path.extension().and_then(|ext| ext.to_str()) == Some("json")
            {
                paths.push(entry_path);
            }
        }
    }

    paths.sort();

    if paths.is_empty() {
        bail!("no JSON fixtures found in {}", path.display());
    }

    Ok(paths)
}

/// Loads one JSON fixture from disk.
pub fn load_fixture(path: &Path) -> anyhow::Result<StatelessValidatorFixture> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read fixture {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to deserialize fixture {}", path.display()))
}

fn unpack_fixture_archive(archive_path: &Path, target_dir: &Path) -> anyhow::Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("failed to open fixture archive {}", archive_path.display()))?;
    let gz = GzDecoder::new(file);
    Archive::new(gz).unpack(target_dir).with_context(|| {
        format!(
            "failed to unpack fixture archive {}",
            archive_path.display()
        )
    })
}

fn is_tar_gz_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| file_name.ends_with(".tar.gz"))
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;

        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use flate2::{Compression, write::GzEncoder};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn prepare_fixture_paths_extracts_tar_gz_archives() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        let fixture_dir = source_dir.join("block");
        fs::create_dir_all(&fixture_dir).unwrap();
        fs::write(fixture_dir.join("example.json"), br#"{"fixture":"ok"}"#).unwrap();
        fs::write(fixture_dir.join("ignore.txt"), b"ignore").unwrap();

        let archive_path = dir.path().join("fixtures.tar.gz");
        let archive_file = File::create(&archive_path).unwrap();
        let encoder = GzEncoder::new(archive_file, Compression::default());
        let mut builder = tar::Builder::new(encoder);
        builder
            .append_path_with_name(fixture_dir.join("example.json"), "block/example.json")
            .unwrap();
        builder
            .append_path_with_name(fixture_dir.join("ignore.txt"), "block/ignore.txt")
            .unwrap();
        builder.finish().unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        let fixture_paths = prepare_fixture_paths(&archive_path).unwrap();

        assert_eq!(fixture_paths.paths.len(), 1);
        assert!(fixture_paths.paths[0].ends_with("example.json"));
        assert_eq!(
            fs::read_to_string(&fixture_paths.paths[0]).unwrap(),
            r#"{"fixture":"ok"}"#
        );
    }
}
