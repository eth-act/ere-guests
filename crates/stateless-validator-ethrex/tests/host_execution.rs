//! Execution tests for `stateless-validator-ethrex` guest program on host

use stateless_validator_ethrex::guest::run_stateless_guest;
use stateless_validator_test::declare_test_host_execution;

declare_test_host_execution!(Ethrex, RpcBpo2, run_stateless_guest);
declare_test_host_execution!(Ethrex, EestGlamsterdamDevnet7, run_stateless_guest);
