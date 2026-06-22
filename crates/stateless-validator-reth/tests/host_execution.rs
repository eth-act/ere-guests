//! Execution tests for `stateless-validator-reth` guest program on host

use stateless_validator_reth::guest::run_stateless_guest;
use stateless_validator_test::declare_test_execution_host;

declare_test_execution_host!(RpcBpo2, run_stateless_guest);
