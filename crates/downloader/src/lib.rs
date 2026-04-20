//! Downloads compiled guests from GitHub releases or action artifacts.

use std::collections::BTreeMap;

use anyhow::{Context, ensure};
use reqwest::{
    Client, ClientBuilder, IntoUrl, RequestBuilder,
    header::{ACCEPT, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tempfile::tempdir;
use tokio::{fs, process::Command};

const REPO_API_URL: &str = "https://api.github.com/repos/eth-act/ere-guests";
const ACTION_NAME: &str = "Compile and Release Compiled Guests";

/// Compiled guest ELF.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompiledGuest {
    /// Raw ELF bytes.
    pub elf: Vec<u8>,
}

#[derive(Clone, Debug)]
enum DownloadSource {
    /// GitHub release assets.
    Tag { assets: BTreeMap<String, String> },
    /// GitHub action artifacts (zipped).
    Rev { artifacts: BTreeMap<String, String> },
}

/// Downloads compiled guests from the `eth-act/ere-guests` repository.
#[derive(Clone, Debug)]
pub struct Downloader {
    client: Client,
    source: DownloadSource,
}

impl Downloader {
    /// Creates a downloader from a GitHub release tag (e.g., `"v0.5.0"`).
    pub async fn from_tag(tag: &str) -> anyhow::Result<Self> {
        let client = github_client(None)?;
        let assets = get_release_assets(&client, tag).await?;
        Ok(Self {
            client,
            source: DownloadSource::Tag { assets },
        })
    }

    /// Creates a downloader from a commit SHA. Requires `github_token`.
    pub async fn from_commit(sha: &str, github_token: &str) -> anyhow::Result<Self> {
        let client = github_client(Some(github_token))?;
        let full_sha = get_full_sha(&client, sha).await?;
        let action_id = get_action_id(&client, &full_sha).await?;
        let artifacts = get_artifacts(&client, action_id).await?;
        Ok(Self {
            client,
            source: DownloadSource::Rev { artifacts },
        })
    }

    /// Downloads the compiled guest by name.
    pub async fn download(&self, guest_name: &str) -> anyhow::Result<CompiledGuest> {
        match &self.source {
            DownloadSource::Tag { assets } => self.download_from_release(assets, guest_name).await,
            DownloadSource::Rev { artifacts } => {
                self.download_from_action(artifacts, guest_name).await
            }
        }
    }

    async fn download_from_release(
        &self,
        assets: &BTreeMap<String, String>,
        guest_name: &str,
    ) -> anyhow::Result<CompiledGuest> {
        let elf_url = assets
            .get(&format!("{guest_name}.elf"))
            .with_context(|| format!("ELF not found: {guest_name}.elf"))?;

        let elf = get_bytes(&self.client, elf_url).await?;

        Ok(CompiledGuest { elf })
    }

    async fn download_from_action(
        &self,
        artifacts: &BTreeMap<String, String>,
        guest_name: &str,
    ) -> anyhow::Result<CompiledGuest> {
        let artifact_url = artifacts
            .get(guest_name)
            .with_context(|| format!("Guest not found: {guest_name}"))?;

        let tempdir = tempdir().context("Failed to create temp dir")?;
        let zip_path = tempdir.path().join("artifact.zip");

        fs::write(&zip_path, get_bytes(&self.client, artifact_url).await?)
            .await
            .context("Failed to write artifact zip")?;

        let output = Command::new("unzip")
            .arg("-o")
            .arg(&zip_path)
            .current_dir(tempdir.path())
            .output()
            .await
            .context("Failed to run unzip")?;
        ensure!(output.status.success(), "Unzip exited with non-zero status");

        let elf = fs::read(tempdir.path().join(format!("{guest_name}.elf")))
            .await
            .with_context(|| format!("Failed to read ELF: {guest_name}.elf"))?;

        Ok(CompiledGuest { elf })
    }
}

fn github_client(token: Option<&str>) -> anyhow::Result<Client> {
    let mut headers: HeaderMap = [
        ("Accept", "application/vnd.github+json"),
        ("X-GitHub-Api-Version", "2022-11-28"),
    ]
    .into_iter()
    .map(|(k, v)| (k.parse().unwrap(), HeaderValue::from_static(v)))
    .collect();

    if let Some(token) = token {
        let mut value = HeaderValue::from_str(&format!("Bearer {token}"))?;
        value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }

    Ok(ClientBuilder::new()
        .user_agent("eth-act/ere-guests")
        .default_headers(headers)
        .build()?)
}

async fn get_release_assets(
    client: &Client,
    tag: &str,
) -> anyhow::Result<BTreeMap<String, String>> {
    #[derive(Deserialize)]
    struct Release {
        assets: Vec<Asset>,
    }

    #[derive(Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
    }

    let url = format!("{REPO_API_URL}/releases/tags/{tag}");
    let Release { assets } = get_json(client, url)
        .await
        .with_context(|| format!("Release not found: {tag}"))?;

    Ok(assets
        .into_iter()
        .map(|a| (a.name, a.browser_download_url))
        .collect())
}

async fn get_full_sha(client: &Client, sha: &str) -> anyhow::Result<String> {
    if sha.len() == 40 {
        return Ok(sha.to_string());
    };

    let url = format!("{REPO_API_URL}/commits/{sha}");
    let accept = HeaderValue::from_static("application/vnd.github.sha");
    let req = client.get(url).header(ACCEPT, accept);
    let res = send(req)
        .await
        .with_context(|| format!("Commit not found: {sha}"))?;
    Ok(res.text().await?)
}

async fn get_action_id(client: &Client, full_sha: &str) -> anyhow::Result<u64> {
    #[derive(Deserialize)]
    struct WorkflowRunsResponse {
        workflow_runs: Vec<WorkflowRun>,
    }

    #[derive(Deserialize)]
    struct WorkflowRun {
        id: u64,
        name: String,
        status: String,
        conclusion: Option<String>,
    }

    let url = format!("{REPO_API_URL}/actions/runs?head_sha={full_sha}");
    let WorkflowRunsResponse { workflow_runs } = get_json(client, url)
        .await
        .with_context(|| format!("Commit not found: {full_sha}"))?;

    workflow_runs
        .into_iter()
        .filter(|run| {
            run.name == ACTION_NAME
                && run.status == "completed"
                && run.conclusion.as_deref() == Some("success")
        })
        .map(|run| run.id)
        .max()
        .with_context(|| format!("No successful release workflow run for commit {full_sha}"))
}

async fn get_artifacts(
    client: &Client,
    action_id: u64,
) -> anyhow::Result<BTreeMap<String, String>> {
    #[derive(Deserialize)]
    struct Artifact {
        name: String,
        archive_download_url: String,
    }

    #[derive(Deserialize)]
    struct ArtifactsResponse {
        artifacts: Vec<Artifact>,
    }

    let url = format!("{REPO_API_URL}/actions/runs/{action_id}/artifacts");
    let ArtifactsResponse { artifacts } = get_json(client, url)
        .await
        .with_context(|| format!("Artifacts not found: {action_id}"))?;

    Ok(artifacts
        .into_iter()
        .map(|artifact| (artifact.name, artifact.archive_download_url))
        .collect())
}

async fn get_json<T: DeserializeOwned>(client: &Client, url: impl IntoUrl) -> anyhow::Result<T> {
    Ok(send(client.get(url)).await?.json().await?)
}

async fn get_bytes(client: &Client, url: impl IntoUrl) -> anyhow::Result<Vec<u8>> {
    Ok(send(client.get(url)).await?.bytes().await?.to_vec())
}

async fn send(builder: RequestBuilder) -> anyhow::Result<reqwest::Response> {
    Ok(builder.send().await?.error_for_status()?)
}

#[cfg(test)]
mod tests {
    use crate::Downloader;

    #[tokio::test]
    async fn download_from_tag() -> anyhow::Result<()> {
        let guest = Downloader::from_tag("v0.5.0")
            .await?
            .download("empty-zisk")
            .await?;
        assert!(!guest.elf.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn download_from_commit() -> anyhow::Result<()> {
        let Ok(github_token) = std::env::var("GITHUB_TOKEN") else {
            return Ok(());
        };

        let guest = Downloader::from_commit("c696d4b", &github_token)
            .await?
            .download("empty-zisk")
            .await?;
        assert!(!guest.elf.is_empty());
        Ok(())
    }
}
