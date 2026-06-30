//! Fixture loading and discovery for the stateless validator debug runner.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use alloy_eips::eip7840::BlobParams;
use anyhow::{Context, bail};
use serde::Deserialize;

/// Deserialized JSON fixture supported by the debug runner.
#[derive(Debug, Clone)]
pub struct StatelessValidatorFixture {
    /// Human-readable fixture identifier.
    pub name: String,
    /// Stateless input bytes.
    pub input_bytes: Vec<u8>,
    /// Expected validation outcome.
    pub success: bool,
    /// Expected serialized guest output, when provided by canonical fixtures.
    pub expected_output_bytes: Option<Vec<u8>>,
}

/// Minimal projection of an EEST `blockchain_test` body — only the fields the
/// debug runner needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct EestStatelessTest {
    network: String,
    config: EestConfig,
    blocks: Vec<EestStatelessBlock>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EestConfig {
    #[serde(rename = "chainid", deserialize_with = "deserialize_hex_u64")]
    chain_id: u64,
    #[serde(default, rename = "blobSchedule")]
    blob_schedule: BTreeMap<String, EestBlobParams>,
}

/// Hex-encoded blob-schedule entry as it appears in EEST fixtures.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlobParams {
    #[serde(deserialize_with = "deserialize_hex_u64")]
    target: u64,
    #[serde(deserialize_with = "deserialize_hex_u64")]
    max: u64,
    #[serde(deserialize_with = "deserialize_hex_u128")]
    base_fee_update_fraction: u128,
}

impl From<&EestBlobParams> for BlobParams {
    fn from(p: &EestBlobParams) -> Self {
        BlobParams {
            target_blob_count: p.target,
            max_blob_count: p.max,
            update_fraction: p.base_fee_update_fraction,
            min_blob_fee: 0,
            max_blobs_per_tx: p.max,
            blob_base_cost: 0,
        }
    }
}

fn deserialize_hex_u64<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let s = String::deserialize(d)?;
    let stripped = s.strip_prefix("0x").unwrap_or(&s);
    u64::from_str_radix(stripped, 16).map_err(serde::de::Error::custom)
}

fn deserialize_hex_u128<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u128, D::Error> {
    let s = String::deserialize(d)?;
    let stripped = s.strip_prefix("0x").unwrap_or(&s);
    u128::from_str_radix(stripped, 16).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestStatelessBlock {
    #[serde(default)]
    stateless_input_bytes: Option<alloy_primitives::Bytes>,
    #[serde(default)]
    stateless_output_bytes: Option<alloy_primitives::Bytes>,
    #[serde(default)]
    expect_exception: Option<String>,
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

    let mut paths = Vec::new();
    collect_json_fixture_paths(path, &mut paths)?;
    paths.sort();

    if paths.is_empty() {
        bail!("no JSON fixtures found in {}", path.display());
    }

    Ok(paths)
}

fn collect_json_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("failed to read fixture directory {}", path.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect path {}", entry_path.display()))?;

        if file_type.is_dir() {
            collect_json_fixture_paths(&entry_path, paths)?;
        } else if file_type.is_file()
            && entry_path.extension().and_then(|ext| ext.to_str()) == Some("json")
        {
            paths.push(entry_path);
        }
    }

    Ok(())
}

/// Loads one or more fixtures from a JSON file. Supports two layouts:
/// - The legacy `{name, stateless_input, success}` shape used by repo fixtures.
/// - The EEST `blockchain_test` shape: a top-level map of test-name → `{network, blocks:
///   [{statelessInputBytes, ...}, ...], ...}`. Each `(test, block)` pair becomes one canonical
///   fixture.
pub fn load_fixtures(path: &Path) -> anyhow::Result<Vec<StatelessValidatorFixture>> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read fixture {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse fixture JSON {}", path.display()))?;

    let map: BTreeMap<String, EestStatelessTest> =
        serde_json::from_value(value).with_context(|| {
            format!(
                "fixture {} is neither a legacy fixture nor an EEST blockchain_test",
                path.display(),
            )
        })?;

    let mut out = Vec::new();
    for (test_name, case) in map {
        for (idx, block) in case.blocks.iter().enumerate() {
            let Some(input_bytes) = &block.stateless_input_bytes else {
                continue;
            };
            if input_bytes.is_empty() {
                continue;
            }
            out.push(StatelessValidatorFixture {
                name: format!("{test_name}#block{idx}"),
                input_bytes: input_bytes.to_vec(),
                success: block.expect_exception.is_none(),
                expected_output_bytes: block
                    .stateless_output_bytes
                    .as_ref()
                    .map(|bytes| bytes.to_vec()),
            });
        }
    }

    if out.is_empty() {
        bail!(
            "no canonical `statelessInputBytes` found in {}; nothing to run",
            path.display()
        );
    }
    Ok(out)
}
