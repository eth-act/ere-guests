//! Execution tests for `stateless-validator-ethrex` guest program on host

use stateless_validator_test::{
    declare_test_host_execution, declare_test_host_recursive_execution,
};

declare_test_host_recursive_execution!(Ethrex);

declare_test_host_execution!(Ethrex, RpcBpo2);
declare_test_host_execution!(Ethrex, RpcGlamsterdamDevnet7);
declare_test_host_execution!(Ethrex, EestGlamsterdamDevnet7);
