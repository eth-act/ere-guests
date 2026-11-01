#![allow(missing_docs)]

/// EIP-8025 forbids the guest from taking the chain identifier or the fork schedule from its
/// private input, so they are associated constants of the guest binary instead. The payload limits
/// come from the same preset and are grouped here for the same reason.
pub trait Network {
    const DEPOSIT_CHAIN_ID: u64;
    const SECONDS_PER_SLOT: u64;
    const MAX_WITHDRAWALS_PER_PAYLOAD: usize;
    const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize;
    const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize;
    const MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD: usize;
    const MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD: usize;
}

#[derive(Debug, Clone, Copy)]
pub struct Mainnet;

impl Network for Mainnet {
    const DEPOSIT_CHAIN_ID: u64 = 1;
    const SECONDS_PER_SLOT: u64 = 12;
    const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
    const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 16;
    const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 2;
    const MAX_BUILDER_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 64;
    const MAX_BUILDER_EXIT_REQUESTS_PER_PAYLOAD: usize = 16;
}
