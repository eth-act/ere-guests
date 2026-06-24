//! Execution tests for `stateless-validator-ethrex` guest program on host

use stateless_validator_ethrex::guest::run_stateless_guest;
use stateless_validator_test::declare_test_host_execution;

declare_test_host_execution!(RpcBpo2, run_stateless_guest);
declare_test_host_execution!(RpcGlamsterdamDevnet5, run_stateless_guest);
declare_test_host_execution!(EestBalDevnet7, run_stateless_guest);
