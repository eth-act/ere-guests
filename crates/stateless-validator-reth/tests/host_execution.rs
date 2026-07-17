//! Execution tests for `stateless-validator-reth` guest program on host

use stateless_validator_reth::guest::run_stateless_guest;
use stateless_validator_test::declare_test_host_execution;

declare_test_host_execution!(RpcBpo2, run_stateless_guest);
// FIXME:
// - 14 EIP-7610
//    - test_init_collision_create_tx
//    - test_init_collision_create_opcode
//    - test_collision_with_create2_revert_in_initcode
//    - test_create2_collision_storage
// - 1 EIP-8037 (stale fixture, should be fixed in execution-specs#2892)
//    - test_creation_tx_regular_check_subtracts_intrinsic_state
// - 1 EIP-8025 (in-block created-code resolution)
//    - test_witness_codes_create_same_hash_then_read
declare_test_host_execution!(EestGlamsterdamDevnet7, run_stateless_guest, failures = 16);
