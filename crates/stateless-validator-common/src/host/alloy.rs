//! Alloy input bridge.
//!
//! Converts alloy execution-layer types into the canonical guest input types.
//! This module is the only place where execution layer dependencies participate
//! in input construction.

use alloy_consensus::{EthereumTxEnvelope, TxEip4844};
use anyhow::{Context, Result};
use libssz_types::SszList;

use crate::guest::input::{
    BlobSchedule, ChainConfig, ExecutionWitness, ForkActivation, ForkConfig, PUBLIC_KEY_BYTES,
    ProtocolFork,
};

/// Recovers public keys from transaction signatures.
pub fn recover_signers<'a, I>(txs: I) -> Result<Vec<[u8; PUBLIC_KEY_BYTES]>>
where
    I: IntoIterator<Item = &'a EthereumTxEnvelope<TxEip4844>>,
{
    txs.into_iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .map(|key| key.to_encoded_point(false).as_bytes().try_into().unwrap())
                .with_context(|| format!("failed to recover signature for tx #{i}"))
        })
        .collect()
}

impl ChainConfig {
    /// Reduces a full [`alloy_genesis::ChainConfig`] to the canonical chain
    /// configuration holding only the fork active at the given block.
    pub fn try_from_alloy(
        chain_config: &alloy_genesis::ChainConfig,
        block_number: u64,
        timestamp: u64,
    ) -> Result<ChainConfig> {
        Ok(ChainConfig {
            chain_id: chain_config.chain_id,
            active_fork: active_fork(chain_config, block_number, timestamp)?,
        })
    }
}

/// Resolves the fork active at the given block along with its activation
/// point and blob schedule.
fn active_fork(
    config: &alloy_genesis::ChainConfig,
    block_number: u64,
    timestamp: u64,
) -> Result<ForkConfig> {
    for (time, name) in [
        (config.bpo3_time, "bpo3"),
        (config.bpo4_time, "bpo4"),
        (config.bpo5_time, "bpo5"),
    ] {
        anyhow::ensure!(time.is_none(), "scheduled fork {name} is not supported");
    }

    let forks = [
        (config.amsterdam_time, ProtocolFork::Amsterdam),
        (config.bpo2_time, ProtocolFork::BPO2),
        (config.bpo1_time, ProtocolFork::BPO1),
        (config.osaka_time, ProtocolFork::Osaka),
        (config.prague_time, ProtocolFork::Prague),
        (config.cancun_time, ProtocolFork::Cancun),
        (config.shanghai_time, ProtocolFork::Shanghai),
        (
            config
                .merge_netsplit_block
                .or(config.terminal_total_difficulty_passed.then_some(0)),
            ProtocolFork::Paris,
        ),
        (config.gray_glacier_block, ProtocolFork::GrayGlacier),
        (config.arrow_glacier_block, ProtocolFork::ArrowGlacier),
        (config.london_block, ProtocolFork::London),
        (config.berlin_block, ProtocolFork::Berlin),
        (config.muir_glacier_block, ProtocolFork::MuirGlacier),
        (config.istanbul_block, ProtocolFork::Istanbul),
        (config.petersburg_block, ProtocolFork::ConstantinopleFix),
        (config.constantinople_block, ProtocolFork::Constantinople),
        (config.byzantium_block, ProtocolFork::Byzantium),
        (
            config.eip158_block.or(config.eip155_block),
            ProtocolFork::SpuriousDragon,
        ),
        (config.eip150_block, ProtocolFork::TangerineWhistle),
        (config.dao_fork_block, ProtocolFork::DAOFork),
        (config.homestead_block, ProtocolFork::Homestead),
    ];
    let (at_block, at_time, fork) = forks
        .into_iter()
        .find_map(|(at, fork)| {
            if fork >= ProtocolFork::Shanghai {
                at.and_then(|at| (timestamp >= at).then_some((None, Some(at), fork)))
            } else {
                at.and_then(|at| (block_number >= at).then_some((Some(at), None, fork)))
            }
        })
        .unwrap_or((Some(0), None, ProtocolFork::Frontier));

    let blob_schedule = if let Some(schedule_key) = schedule_key(fork) {
        let params = config
            .blob_schedule
            .get(schedule_key)
            .with_context(|| format!("missing blob schedule entry for {schedule_key}"))?;
        Some(BlobSchedule {
            target: params.target_blob_count,
            max: params.max_blob_count,
            base_fee_update_fraction: u64::try_from(params.update_fraction)
                .context("blob base fee update fraction exceeds u64")?,
        })
    } else {
        None
    };

    Ok(ForkConfig::new(
        fork,
        ForkActivation::new(at_block, at_time),
        blob_schedule,
    ))
}

fn schedule_key(fork: ProtocolFork) -> Option<&'static str> {
    Some(match fork {
        // The capitalized key matches the spelling that
        // `blob_schedule_blob_params` of alloy-genesis expects, where every
        // other fork key is lowercase.
        ProtocolFork::Amsterdam => "Amsterdam",
        ProtocolFork::BPO2 => "bpo2",
        ProtocolFork::BPO1 => "bpo1",
        ProtocolFork::Osaka => "osaka",
        ProtocolFork::Prague => "prague",
        ProtocolFork::Cancun => "cancun",
        _ => return None,
    })
}

impl ExecutionWitness {
    /// Converts an alloy execution witness into the canonical witness. The
    /// canonical witness has no keys field because stateless validation does not
    /// consume it.
    pub fn try_from_alloy(
        witness: &alloy_rpc_types_debug::ExecutionWitness,
    ) -> Result<ExecutionWitness> {
        Ok(ExecutionWitness {
            state: ssz_bytes_list(&witness.state, "witness state")?,
            codes: ssz_bytes_list(&witness.codes, "witness codes")?,
            headers: ssz_bytes_list(&witness.headers, "witness headers")?,
        })
    }
}

fn ssz_bytes_list<const M: usize, const N: usize>(
    items: &[alloy_primitives::Bytes],
    label: &str,
) -> Result<SszList<SszList<u8, M>, N>> {
    let list = items
        .iter()
        .map(|item| SszList::try_from(item.to_vec()))
        .collect::<Result<_, _>>()
        .map_err(|err| anyhow::anyhow!("{label} item length should be within bounds: {err:?}"))?;
    ssz_list(list, label)
}

fn ssz_list<T, const N: usize>(values: Vec<T>, label: &str) -> Result<SszList<T, N>> {
    SszList::try_from(values)
        .map_err(|err| anyhow::anyhow!("{label} length should be within bounds: {err:?}"))
}

#[cfg(test)]
mod tests {
    use alloy_eips::eip7840::BlobParams;
    use alloy_genesis::ChainConfig;

    use crate::guest::input::{ChainConfig as CanonicalChainConfig, ProtocolFork};

    /// A merged config that schedules each blob-bearing fork one second apart.
    /// Resolving at a fork's activation timestamp selects that fork along with
    /// the blob schedule registered under its `schedule_key`, where Amsterdam is
    /// the capitalized key.
    #[test]
    fn resolves_active_time_fork_with_its_blob_schedule() {
        let schedule = [
            (1u64, ProtocolFork::Cancun, "cancun", BlobParams::cancun()),
            (2, ProtocolFork::Prague, "prague", BlobParams::prague()),
            (3, ProtocolFork::Osaka, "osaka", BlobParams::osaka()),
            (4, ProtocolFork::BPO1, "bpo1", BlobParams::bpo1()),
            (5, ProtocolFork::BPO2, "bpo2", BlobParams::bpo2()),
            (6, ProtocolFork::Amsterdam, "Amsterdam", BlobParams::bpo2()),
        ];

        let mut config = ChainConfig {
            terminal_total_difficulty_passed: true,
            shanghai_time: Some(0),
            cancun_time: Some(1),
            prague_time: Some(2),
            osaka_time: Some(3),
            bpo1_time: Some(4),
            bpo2_time: Some(5),
            amsterdam_time: Some(6),
            ..Default::default()
        };
        for (_, _, key, params) in schedule {
            config.blob_schedule.insert(key.to_string(), params);
        }

        for (timestamp, fork, _, params) in schedule {
            let active = CanonicalChainConfig::try_from_alloy(&config, 0, timestamp)
                .unwrap()
                .active_fork;

            assert_eq!(active.fork, fork);
            assert_eq!(active.activation.timestamp(), Some(timestamp));
            assert_eq!(active.activation.block_number(), None);

            let blob_schedule = active.blob_schedule().expect("blob schedule present");
            assert_eq!(blob_schedule.target, params.target_blob_count);
            assert_eq!(blob_schedule.max, params.max_blob_count);
            assert_eq!(
                u128::from(blob_schedule.base_fee_update_fraction),
                params.update_fraction
            );
        }
    }

    /// A pre-Shanghai block resolves by block number with no blob schedule.
    #[test]
    fn resolves_block_activated_fork_without_blob_schedule() {
        let config = ChainConfig {
            homestead_block: Some(0),
            london_block: Some(100),
            ..Default::default()
        };
        let active = CanonicalChainConfig::try_from_alloy(&config, 150, 0)
            .unwrap()
            .active_fork;
        assert_eq!(active.fork, ProtocolFork::London);
        assert_eq!(active.activation.block_number(), Some(100));
        assert_eq!(active.activation.timestamp(), None);
        assert!(active.blob_schedule().is_none());
    }
}
