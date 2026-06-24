//! Execution tests for `stateless-validator-reth` guest program on host

use stateless_validator_reth::guest::run_stateless_guest;
use stateless_validator_test::declare_test_host_execution;

declare_test_host_execution!(RpcBpo2, run_stateless_guest);
declare_test_host_execution!(RpcGlamsterdamDevnet5, run_stateless_guest);
// FIXME: 14 EIP-7610 failures, 1 EIP-8037 failure, 1 EIP-8025 failure
declare_test_host_execution!(EestBalDevnet7, run_stateless_guest, should_panic);
