//! Fixture loading and discovery for the stateless validator.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use alloy_primitives::Bytes;
use rayon::prelude::*;
use serde::Deserialize;
use tar::Archive;
use tracing::info;
use walkdir::{DirEntry, WalkDir};

/// Release hosting the EEST fixtures filled by `ethereum/execution-specs`.
const EEST_FIXTURES_BASE_URL: &str =
    "https://github.com/ethereum/execution-specs/releases/download/tests-zkevm@v0.6.2";
/// Release hosting the RPC-derived fixtures from `witness-generator-spec-cli`.
const RPC_FIXTURES_BASE_URL: &str =
    "https://github.com/han0110/ere-guests/releases/download/rpc-fixtures@v0.1.0";

/// A preset fixture set identifying both its source archive and its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixturePreset {
    /// RPC-derived fixtures from the `mainnet`.
    RpcBpo2,
    /// RPC-derived fixtures from the `glamsterdam-devnet-5`.
    RpcGlamsterdamDevnet5,
    /// EEST `blockchain_test` fixtures based on `bal-devnet-7`.
    EestBalDevnet7,
}

/// Download and on-disk layout details for a [`FixturePreset`].
struct FixtureSource {
    /// URL of the release archive.
    url: String,
    /// Subdirectory under `<crate>/fixtures/`.
    dir: &'static str,
    /// Subdirectory inside the unpacked archive holding the fixtures.
    archive_dir: &'static str,
}

impl FixturePreset {
    fn source(self) -> FixtureSource {
        match self {
            Self::EestBalDevnet7 => FixtureSource {
                url: format!("{EEST_FIXTURES_BASE_URL}/fixtures_zkevm.tar.gz"),
                dir: "eest-bal-devnet-7",
                archive_dir: "fixtures/blockchain_tests",
            },
            Self::RpcBpo2 => FixtureSource {
                url: format!("{RPC_FIXTURES_BASE_URL}/rpc-bpo2.tar.zst"),
                dir: "rpc-bpo2",
                archive_dir: "rpc-bpo2",
            },
            Self::RpcGlamsterdamDevnet5 => FixtureSource {
                url: format!("{RPC_FIXTURES_BASE_URL}/rpc-glamsterdam-devnet-5.tar.zst"),
                dir: "rpc-glamsterdam-devnet-5",
                archive_dir: "rpc-glamsterdam-devnet-5",
            },
        }
    }
}

/// A fixture normalized to canonical schema-prefixed SSZ input and output bytes.
#[derive(Debug, Clone)]
pub struct StatelessValidatorFixture {
    /// Human-readable identifier.
    pub name: String,
    /// Whether the block is expected to validate successfully.
    pub success: bool,
    /// Canonical schema-prefixed SSZ input bytes consumed by the guests.
    pub stateless_input_bytes: Vec<u8>,
    /// Expected serialized guest output bytes.
    pub stateless_output_bytes: Vec<u8>,
}

/// Returns every fixture of `preset`, downloading and unpacking its archive into
/// the local cache on first use. Fixtures are sorted by name for determinism.
pub fn preset_fixtures(preset: FixturePreset) -> Vec<StatelessValidatorFixture> {
    let mut fixtures = WalkDir::new(ensure_preset(preset))
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .filter(is_fixture_file)
        .flat_map(|entry| load_fixtures(entry.path()))
        .collect::<Vec<_>>();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

/// Returns whether `entry` is a recognised fixture file, namely a `.json` or
/// zstd-compressed `.json.zst` file.
fn is_fixture_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".json") || name.ends_with(".json.zst"))
}

/// Loads every fixture from a single JSON file, transparently decompressing a
/// `.zst` file and auto-detecting the EEST or RPC layout.
pub fn load_fixtures(path: impl AsRef<Path>) -> Vec<StatelessValidatorFixture> {
    let path = path.as_ref();
    let bytes = fs::read(path).unwrap();
    let bytes = if path.extension().is_some_and(|ext| ext == "zst") {
        zstd::stream::decode_all(bytes.as_slice()).unwrap()
    } else {
        bytes
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    if value.get("statelessInputBytes").is_some() {
        let rpc: RpcFixture = serde_json::from_value(value).unwrap();
        return vec![StatelessValidatorFixture {
            name: format!("rpc-{}-{}", rpc.network, rpc.block_number),
            success: true,
            stateless_input_bytes: rpc.stateless_input_bytes.to_vec(),
            stateless_output_bytes: rpc.stateless_output_bytes.to_vec(),
        }];
    }

    let tests: EestFixture = serde_json::from_value(value).unwrap();
    tests
        .into_iter()
        .flat_map(|(test_id, test)| {
            test.blocks
                .into_iter()
                .enumerate()
                .filter_map(move |(idx, block)| {
                    let (input, output) = block
                        .stateless_input_bytes
                        .zip(block.stateless_output_bytes)?;
                    (!input.is_empty()).then(|| StatelessValidatorFixture {
                        name: format!("{test_id}#block{idx}"),
                        success: block.expect_exception.is_none(),
                        stateless_input_bytes: input.to_vec(),
                        stateless_output_bytes: output.to_vec(),
                    })
                })
        })
        .collect()
}

/// Ensures the cached fixture directory for `preset` exists, downloading and
/// unpacking the release archive when missing. Returns the directory.
fn ensure_preset(preset: FixturePreset) -> PathBuf {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());

    let source = preset.source();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(source.dir);
    if !dir.exists() {
        download_and_unpack(&source, &dir);
    }
    dir
}

/// Downloads the preset's release archive and moves its `archive_dir`
/// subdirectory to `dir`, discarding the rest of the archive.
fn download_and_unpack(source: &FixtureSource, dir: &Path) {
    info!("Downloading fixture archive {}", source.url);
    let bytes = reqwest::blocking::get(&source.url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap();

    fs::create_dir_all(dir.parent().unwrap()).unwrap();
    let tempdir = tempfile::tempdir_in(dir.parent().unwrap()).unwrap();
    if source.url.ends_with(".tar.gz") {
        Archive::new(flate2::read::GzDecoder::new(&bytes[..]))
            .unpack(tempdir.path())
            .unwrap();
    } else if source.url.ends_with(".tar.zst") {
        Archive::new(zstd::stream::read::Decoder::new(&bytes[..]).unwrap())
            .unpack(tempdir.path())
            .unwrap();
    } else {
        unreachable!()
    }

    fs::rename(tempdir.path().join(source.archive_dir), dir).unwrap();
}

/// Wire shape of an RPC artifact produced by `witness-generator-spec-cli`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcFixture {
    network: String,
    block_number: u64,
    stateless_input_bytes: Bytes,
    stateless_output_bytes: Bytes,
}

type EestFixture = BTreeMap<String, EestTest>;

/// Minimal projection of an EEST `blockchain_test` body.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestTest {
    blocks: Vec<EestBlock>,
}

/// Minimal projection of a single EEST block.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlock {
    stateless_input_bytes: Option<Bytes>,
    stateless_output_bytes: Option<Bytes>,
    expect_exception: Option<String>,
}
