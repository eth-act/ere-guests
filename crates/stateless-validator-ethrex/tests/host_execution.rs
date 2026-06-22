//! Execution tests for `stateless-validator-ethrex` guest program on host

use stateless_validator_ethrex::guest::run_stateless_guest;
use stateless_validator_test::declare_test_execution_host;

declare_test_execution_host!(RpcBpo2, run_stateless_guest);
declare_test_execution_host!(RpcGlamsterdamDevnet5, run_stateless_guest);
declare_test_execution_host!(EestBalDevnet7, run_stateless_guest);
