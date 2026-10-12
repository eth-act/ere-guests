//! Downloads the guests registered in `artifact-registry.json` and checks their sha256.

use anyhow::{Context, ensure};
use ere_catalog::zkVMKind;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use stateless_validator_catalog::StatelessValidatorKind;

const ARTIFACT_REGISTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../artifact-registry.json"
));

/// Registered guest ELF and Program VK.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompiledGuest {
    /// Raw ELF bytes.
    pub elf: Vec<u8>,
    /// Raw Program VK bytes.
    pub program_vk: Vec<u8>,
}

/// Guest of one zkVM in `artifact-registry.json`.
#[derive(Debug, Deserialize)]
struct Artifact {
    zkvm: String,
    zkvm_version: String,
    elf_url: String,
    elf_sha256: String,
    vk_url: String,
    vk_sha256: String,
}

/// Downloads the guest of `stateless_validator_kind` on `zkvm_kind` that this crate's
/// `artifact-registry.json` registers. Fails unless the ELF and Program VK match the registered
/// sha256.
pub async fn download(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> anyhow::Result<CompiledGuest> {
    let artifact = registered_artifact(stateless_validator_kind, zkvm_kind)?;
    let artifact_name = format!(
        "stateless-validator-{stateless_validator_kind}-{zkvm_kind}-{}",
        artifact.zkvm_version
    );
    let client = Client::builder().user_agent("eth-act/ere-guests").build()?;
    let elf = get_verified_bytes(&client, &artifact.elf_url, &artifact.elf_sha256)
        .await
        .with_context(|| format!("Failed to download ELF: {artifact_name}.elf"))?;
    let program_vk = get_verified_bytes(&client, &artifact.vk_url, &artifact.vk_sha256)
        .await
        .with_context(|| format!("Failed to download Program VK: {artifact_name}.vk"))?;
    Ok(CompiledGuest { elf, program_vk })
}

fn registered_artifact(
    stateless_validator_kind: StatelessValidatorKind,
    zkvm_kind: zkVMKind,
) -> anyhow::Result<Artifact> {
    #[derive(Deserialize)]
    struct ArtifactRegistry {
        stateless_validators: Vec<StatelessValidator>,
    }

    #[derive(Deserialize)]
    struct StatelessValidator {
        name: String,
        artifacts: Vec<Artifact>,
    }

    let registry: ArtifactRegistry = serde_json::from_str(ARTIFACT_REGISTRY_JSON)?;
    registry
        .stateless_validators
        .into_iter()
        .find(|validator| validator.name == stateless_validator_kind.as_str())
        .and_then(|validator| {
            validator
                .artifacts
                .into_iter()
                .find(|artifact| artifact.zkvm == zkvm_kind.as_str())
        })
        .with_context(|| {
            format!("{stateless_validator_kind}-{zkvm_kind} not found in artifact-registry.json")
        })
}

/// Downloads `url` and fails unless its sha256 is `sha256`.
async fn get_verified_bytes(client: &Client, url: &str, sha256: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    verify_sha256(&bytes, sha256)?;
    Ok(bytes.to_vec())
}

fn verify_sha256(bytes: &[u8], expected: &str) -> anyhow::Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    ensure!(
        actual == expected,
        "sha256 is {actual}, but artifact-registry.json registers {expected}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use ere_catalog::zkVMKind;
    use stateless_validator_catalog::StatelessValidatorKind;

    use crate::{download, registered_artifact, verify_sha256};

    #[test]
    fn resolves_artifact_version_from_registry() -> anyhow::Result<()> {
        let registered = [
            (
                StatelessValidatorKind::Ethrex,
                zkVMKind::OpenVM,
                "v2.1.0-preview",
            ),
            (StatelessValidatorKind::Ethrex, zkVMKind::SP1, "v6.6.0"),
            (
                StatelessValidatorKind::Ethrex,
                zkVMKind::Zisk,
                "v1.2.0-alpha",
            ),
            (
                StatelessValidatorKind::Reth,
                zkVMKind::OpenVM,
                "v2.1.0-preview",
            ),
            (StatelessValidatorKind::Reth, zkVMKind::SP1, "v6.6.0"),
            (StatelessValidatorKind::Reth, zkVMKind::Zisk, "v1.2.0-alpha"),
            (StatelessValidatorKind::Zesu, zkVMKind::Zisk, "v1.2.0-alpha"),
            (
                StatelessValidatorKind::Nimbus,
                zkVMKind::Zisk,
                "v1.2.0-alpha",
            ),
        ];
        for kind in StatelessValidatorKind::iter() {
            for zkvm_kind in [zkVMKind::OpenVM, zkVMKind::SP1, zkVMKind::Zisk] {
                match registered
                    .iter()
                    .find(|&&(registered_kind, registered_zkvm, _)| {
                        (registered_kind, registered_zkvm) == (kind, zkvm_kind)
                    }) {
                    Some(&(_, _, expected)) => {
                        assert_eq!(registered_artifact(kind, zkvm_kind)?.zkvm_version, expected)
                    }
                    None => assert_eq!(
                        registered_artifact(kind, zkvm_kind)
                            .unwrap_err()
                            .to_string(),
                        format!("{kind}-{zkvm_kind} not found in artifact-registry.json")
                    ),
                }
            }
        }
        Ok(())
    }

    #[test]
    fn rejects_sha256_mismatch() {
        let empty_sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(verify_sha256(b"", empty_sha256).is_ok());
        assert_eq!(
            verify_sha256(b"elf", empty_sha256).unwrap_err().to_string(),
            "sha256 is 780d84b20d7ae7e6292919399348bdbf96025270136198083fc8a4da398b5ca9, but \
             artifact-registry.json registers \
             e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn downloads_registered_guest() -> anyhow::Result<()> {
        download(StatelessValidatorKind::Ethrex, zkVMKind::OpenVM).await?;
        Ok(())
    }
}
