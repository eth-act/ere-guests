//! Fixture loading and discovery for release-backed stateless validators.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use alloy_primitives::Bytes;
use rayon::prelude::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;
use tracing::info;
use walkdir::{DirEntry, WalkDir};

const EEST_FIXTURES_URL: &str = "https://github.com/ethereum/execution-specs/releases/download/tests-zkevm@v0.8.2/fixtures_zkevm.tar.gz";
const EEST_FIXTURES_SHA256: &str =
    "c58fbe493c1c37ab8371fd0ebb4ded668c08daf774f7f2fb798f6e7939810155";
const DEVNET_PRESET_BATCH: &str = "93350-93359";
const DEVNET_PRESET_ARTIFACT_COUNT: usize = 10;
const DEVNET_PRESET_SHA256: &str =
    "dfcfc4520b59404e2ad6baa621d1b0b8626cd726904ca84c536c235ee6098811";

/// Name of the rolling execution-layer devnet fixture set.
pub const DEVNET_NAME: &str = "glamsterdam-devnet-8";
/// R2 bucket that hosts the devnet-8 fixtures and batch index.
pub const DEVNET_FIXTURES_BASE_URL: &str =
    "https://pub-760ad8b3dd9547539f829c1ea30f18b5.r2.dev/devnets/glamsterdam-devnet-8";

/// A fixture normalized to canonical schema-prefixed SSZ input and fixed-size output bytes.
#[derive(Debug, Clone)]
pub struct StatelessValidatorFixture {
    /// Human-readable identifier.
    pub name: String,
    /// Canonical schema-prefixed SSZ input bytes consumed by the guest.
    pub stateless_input_bytes: Vec<u8>,
    /// Expected serialized guest output bytes.
    pub stateless_output_bytes: Vec<u8>,
}

/// Returns all `tests-zkevm@v0.8.2` fixtures, downloading them on first use.
pub fn eest_fixtures() -> Vec<StatelessValidatorFixture> {
    archive_fixtures(
        "eest-tests-zkevm-v0.8.2",
        EEST_FIXTURES_URL,
        "fixtures/blockchain_tests",
        Some(EEST_FIXTURES_SHA256),
    )
}

/// Returns the pinned 10-block devnet-8 fixture set used by pull request tests.
pub fn devnet_preset_fixtures() -> Vec<StatelessValidatorFixture> {
    let fixtures = archive_fixtures(
        &format!("rpc-{DEVNET_NAME}/{DEVNET_PRESET_BATCH}"),
        &format!("{DEVNET_FIXTURES_BASE_URL}/exports/batches/{DEVNET_PRESET_BATCH}.tar.zst"),
        "blockchain_tests",
        Some(DEVNET_PRESET_SHA256),
    );
    assert_eq!(
        fixtures.len(),
        DEVNET_PRESET_ARTIFACT_COUNT,
        "devnet preset {DEVNET_PRESET_BATCH} must contain {DEVNET_PRESET_ARTIFACT_COUNT} fixtures"
    );
    fixtures
}

/// Returns the latest `count` devnet-8 block fixtures from the rolling batch catalog.
pub fn latest_devnet_fixtures(count: usize) -> Vec<StatelessValidatorFixture> {
    assert!(count > 0, "devnet fixture count must be positive");
    let batches = latest_devnet_batches(&fetch_devnet_batches().unwrap(), count).unwrap();
    let mut fixtures = devnet_fixtures(&batches);
    fixtures.drain(..fixtures.len().saturating_sub(count));
    fixtures
}

/// Returns the devnet-8 block fixtures from the selected batches.
pub fn devnet_fixtures(batches: &[DevnetBatch]) -> Vec<StatelessValidatorFixture> {
    batches
        .iter()
        .flat_map(|batch| {
            archive_fixtures(
                &format!(
                    "rpc-{DEVNET_NAME}/{}-{}",
                    batch.batch_start_block, batch.batch_end_block
                ),
                &format!("{DEVNET_FIXTURES_BASE_URL}/{}", batch.path),
                "blockchain_tests",
                Some(batch.sha256.trim_start_matches("0x")),
            )
        })
        .collect()
}

/// Returns every fixture in `archive_dir`, caching its verified source archive locally.
pub fn archive_fixtures(
    dir: &str,
    url: &str,
    archive_dir: &str,
    sha256: Option<&str>,
) -> Vec<StatelessValidatorFixture> {
    load_fixtures_from_dir(ensure_fixtures(dir, url, archive_dir, sha256))
}

fn is_json_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".json"))
}

fn load_fixtures_from_dir(dir: impl AsRef<Path>) -> Vec<StatelessValidatorFixture> {
    let mut fixtures = WalkDir::new(dir)
        .into_iter()
        .par_bridge()
        .filter_map(Result::ok)
        .filter(is_json_file)
        .flat_map(|entry| load_fixtures_from_file(entry.path()))
        .collect::<Vec<_>>();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

/// Loads every stateless fixture from one EEST `blockchain_test` JSON file.
pub fn load_fixtures_from_file(path: impl AsRef<Path>) -> Vec<StatelessValidatorFixture> {
    let bytes = fs::read(path).unwrap();
    let tests: EestFixture = serde_json::from_slice(&bytes).unwrap();
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
                        stateless_input_bytes: input.to_vec(),
                        stateless_output_bytes: output.to_vec(),
                    })
                })
        })
        .collect()
}

fn ensure_fixtures(dir: &str, url: &str, archive_dir: &str, sha256: Option<&str>) -> PathBuf {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|error| error.into_inner());

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(dir);
    if !dir.exists() {
        download_and_unpack(url, archive_dir, &dir, sha256);
    }
    dir
}

fn download_and_unpack(url: &str, archive_dir: &str, dir: &Path, sha256: Option<&str>) {
    info!("Downloading fixture archive {url}");
    let bytes = reqwest::blocking::get(url)
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .unwrap();
    if let Some(sha256) = sha256 {
        assert_eq!(
            const_hex::encode(Sha256::digest(&bytes)),
            sha256,
            "fixture archive checksum mismatch for {url}"
        );
    }

    fs::create_dir_all(dir.parent().unwrap()).unwrap();
    let tempdir = tempfile::tempdir_in(dir.parent().unwrap()).unwrap();
    if url.ends_with(".tar.gz") {
        Archive::new(flate2::read::GzDecoder::new(&bytes[..]))
            .unpack(tempdir.path())
            .unwrap();
    } else if url.ends_with(".tar.zst") {
        Archive::new(zstd::stream::read::Decoder::new(&bytes[..]).unwrap())
            .unpack(tempdir.path())
            .unwrap();
    } else {
        panic!("unsupported fixture archive extension: {url}");
    }
    fs::rename(tempdir.path().join(archive_dir), dir).unwrap();
}

/// Downloads the devnet batch index, which lists every published block batch.
pub fn fetch_devnet_batches() -> anyhow::Result<String> {
    let url = format!("{DEVNET_FIXTURES_BASE_URL}/batches.jsonl");
    info!("Downloading devnet batch index {url}");
    Ok(reqwest::blocking::get(&url)?.error_for_status()?.text()?)
}

fn latest_devnet_batches(index: &str, count: usize) -> anyhow::Result<Vec<DevnetBatch>> {
    anyhow::ensure!(count > 0, "devnet fixture count must be positive");
    let mut batches = devnet_batches(index)?;
    let take = (batches
        .iter()
        .rev()
        .scan(0, |artifacts, batch| {
            *artifacts += batch.artifact_count;
            Some(*artifacts)
        })
        .take_while(|artifacts| *artifacts < count)
        .count()
        + 1)
    .min(batches.len());
    Ok(batches.split_off(batches.len() - take))
}

/// Parses the batch index in the order it lists, from the oldest batch to the newest.
pub fn devnet_batches(index: &str) -> anyhow::Result<Vec<DevnetBatch>> {
    let batches = index
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<DevnetBatch>)
        .collect::<Result<Vec<_>, _>>()?;
    anyhow::ensure!(!batches.is_empty(), "devnet batch index is empty");
    Ok(batches)
}

type EestFixture = BTreeMap<String, EestTest>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestTest {
    blocks: Vec<EestBlock>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlock {
    stateless_input_bytes: Option<Bytes>,
    stateless_output_bytes: Option<Bytes>,
}

/// One published archive of consecutive devnet blocks.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevnetBatch {
    /// First block in the archive.
    pub batch_start_block: u64,
    /// Last block in the archive.
    pub batch_end_block: u64,
    /// Number of blocks in the archive.
    pub artifact_count: usize,
    /// SHA-256 of the archive.
    pub sha256: String,
    /// Path of the archive, relative to the fixture base URL.
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::latest_devnet_batches;

    const INDEX: &str = r#"
{"batchStartBlock":1,"batchEndBlock":10,"artifactCount":10,"sha256":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","path":"1-10.tar.zst"}
{"batchStartBlock":11,"batchEndBlock":20,"artifactCount":10,"sha256":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","path":"11-20.tar.zst"}
{"batchStartBlock":21,"batchEndBlock":30,"artifactCount":10,"sha256":"0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","path":"21-30.tar.zst"}
{"batchStartBlock":31,"batchEndBlock":40,"artifactCount":10,"sha256":"0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd","path":"31-40.tar.zst"}
{"batchStartBlock":41,"batchEndBlock":50,"artifactCount":10,"sha256":"0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","path":"41-50.tar.zst"}
{"batchStartBlock":51,"batchEndBlock":60,"artifactCount":10,"sha256":"0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","path":"51-60.tar.zst"}
"#;

    #[test]
    fn selects_latest_batches_covering_requested_count() {
        let batches = latest_devnet_batches(INDEX, 15).unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].batch_start_block, 41);
        assert_eq!(batches[1].batch_end_block, 60);
    }

    #[test]
    fn selects_all_available_batches_when_request_exceeds_catalog() {
        let batches = latest_devnet_batches(INDEX, 100).unwrap();
        assert_eq!(batches.len(), 6);
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.artifact_count)
                .sum::<usize>(),
            60
        );
        assert_eq!(batches[0].batch_start_block, 1);
        assert_eq!(batches[5].batch_end_block, 60);
    }

    #[test]
    fn rejects_empty_or_malformed_batch_index() {
        assert!(latest_devnet_batches("", 1).is_err());
        assert!(latest_devnet_batches("not-json", 1).is_err());
        assert!(latest_devnet_batches(INDEX, 0).is_err());
    }
}
