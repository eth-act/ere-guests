//! Implementations for host environment.

use alloy_eips::eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS;
use ere_zkvm_interface::Input;
use ethrex_common::{
    H160,
    types::{
        BlobSchedule, ChainConfig, ForkBlobSchedule,
        block_execution_witness::{self, RpcExecutionWitness},
    },
};
use guest::{GuestIo, Io};
use stateless_validator_reth::guest::StatelessValidatorRethInput;

use crate::guest::{StatelessValidatorEthrexGuest, StatelessValidatorEthrexInput};

#[rustfmt::skip]
pub use {
    ethrex_guest_program::input::ProgramInput,
    stateless::StatelessInput,
};

impl StatelessValidatorEthrexInput {
    /// Construct [`StatelessValidatorEthrexInput`] given [`StatelessInput`].
    pub fn new(stateless_input: &StatelessInput, valid_block: bool) -> anyhow::Result<Self> {
        let reth_input = StatelessValidatorRethInput::new(stateless_input, valid_block)?;
        let new_payload_request = reth_input.new_payload_request;

        Ok(Self {
            new_payload_request,
            execution_witness: from_reth_witness_to_ethrex_witness(
                stateless_input.block.number,
                stateless_input,
            )?,
        })
    }

    /// Returns [`Input`] to [`zkVM`] methods.
    ///
    /// [`zkVM`]: ere_zkvm_interface::zkVM
    pub fn to_zkvm_input(&self) -> anyhow::Result<Input> {
        let stdin = GuestIo::<StatelessValidatorEthrexGuest>::serialize_input(self)?;
        Ok(Input::new().with_prefixed_stdin(stdin))
    }
}

fn from_reth_witness_to_ethrex_witness(
    block_number: u64,
    stateless_input: &StatelessInput,
) -> anyhow::Result<block_execution_witness::ExecutionWitness> {
    let codes = stateless_input
        .witness
        .codes
        .iter()
        .map(|b| b.to_vec().into())
        .collect();
    let block_headers_bytes = stateless_input
        .witness
        .headers
        .iter()
        .map(|h| h.to_vec().into())
        .collect();

    let chain_config = ChainConfig {
        chain_id: stateless_input.chain_config.chain_id,
        homestead_block: stateless_input.chain_config.homestead_block,
        dao_fork_block: stateless_input.chain_config.dao_fork_block,
        dao_fork_support: stateless_input.chain_config.dao_fork_support,
        eip150_block: stateless_input.chain_config.eip150_block,
        eip155_block: stateless_input.chain_config.eip155_block,
        eip158_block: stateless_input.chain_config.eip158_block,
        byzantium_block: stateless_input.chain_config.byzantium_block,
        constantinople_block: stateless_input.chain_config.constantinople_block,
        petersburg_block: stateless_input.chain_config.petersburg_block,
        istanbul_block: stateless_input.chain_config.istanbul_block,
        muir_glacier_block: stateless_input.chain_config.muir_glacier_block,
        berlin_block: stateless_input.chain_config.berlin_block,
        london_block: stateless_input.chain_config.london_block,
        arrow_glacier_block: stateless_input.chain_config.arrow_glacier_block,
        gray_glacier_block: stateless_input.chain_config.gray_glacier_block,
        merge_netsplit_block: stateless_input.chain_config.merge_netsplit_block,
        shanghai_time: stateless_input.chain_config.shanghai_time,
        cancun_time: stateless_input.chain_config.cancun_time,
        prague_time: stateless_input.chain_config.prague_time,
        verkle_time: None,
        osaka_time: stateless_input.chain_config.osaka_time,
        terminal_total_difficulty: stateless_input
            .chain_config
            .terminal_total_difficulty
            .map(|ttd| TryInto::<u128>::try_into(ttd).unwrap()),
        terminal_total_difficulty_passed: stateless_input
            .chain_config
            .terminal_total_difficulty_passed,
        blob_schedule: BlobSchedule {
            cancun: get_blob_schedule(&stateless_input.chain_config, "cancun")
                .unwrap_or_else(|| BlobSchedule::default().cancun),
            prague: get_blob_schedule(&stateless_input.chain_config, "prague")
                .unwrap_or_else(|| BlobSchedule::default().prague),
            osaka: get_blob_schedule(&stateless_input.chain_config, "osaka")
                .unwrap_or_else(|| BlobSchedule::default().osaka),
            bpo1: get_blob_schedule(&stateless_input.chain_config, "bpo1")
                .unwrap_or_else(|| BlobSchedule::default().bpo1),
            bpo2: get_blob_schedule(&stateless_input.chain_config, "bpo2")
                .unwrap_or_else(|| BlobSchedule::default().bpo2),
            bpo3: get_blob_schedule(&stateless_input.chain_config, "bpo3"),
            bpo4: get_blob_schedule(&stateless_input.chain_config, "bpo4"),
            bpo5: get_blob_schedule(&stateless_input.chain_config, "bpo5"),
            amsterdam: get_blob_schedule(&stateless_input.chain_config, "amsterdam"),
        },
        deposit_contract_address: stateless_input
            .chain_config
            .deposit_contract_address
            .map(|addr| H160::from_slice(addr.as_slice()))
            .unwrap_or_else(|| H160::from_slice(MAINNET_DEPOSIT_CONTRACT_ADDRESS.as_slice())),
        bpo1_time: stateless_input.chain_config.bpo1_time,
        bpo2_time: stateless_input.chain_config.bpo2_time,
        bpo3_time: stateless_input.chain_config.bpo3_time,
        bpo4_time: stateless_input.chain_config.bpo4_time,
        bpo5_time: stateless_input.chain_config.bpo5_time,
        amsterdam_time: None,
        enable_verkle_at_genesis: false,
    };

    let nodes = stateless_input
        .witness
        .state
        .iter()
        .map(|node_rlp| node_rlp.to_vec().into())
        .collect();

    let keys = stateless_input
        .witness
        .keys
        .iter()
        .map(|k| k.to_vec().into())
        .collect();

    let rpc_witness = RpcExecutionWitness {
        state: nodes,
        keys,
        codes,
        headers: block_headers_bytes,
    };

    Ok(rpc_witness.into_execution_witness(chain_config, block_number)?)
}

fn get_blob_schedule(
    chain_config: &alloy_genesis::ChainConfig,
    name: &str,
) -> Option<ethrex_common::types::ForkBlobSchedule> {
    chain_config
        .blob_schedule
        .get(name)
        .map(|s| ForkBlobSchedule {
            // Reth and Ethrex have some mismatched data type representations. Reth uses bigger ints.
            // Downcasting should never cause an overflow, but let's be safe and panic if this ever happens.
            base_fee_update_fraction: s.update_fraction.try_into().unwrap(),
            target: s.target_blob_count.try_into().unwrap(),
            max: s.max_blob_count.try_into().unwrap(),
        })
}
