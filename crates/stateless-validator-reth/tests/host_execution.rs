//! Execution tests for `stateless-validator-reth` guest program on host

use stateless_validator_test::declare_test_host_execution;

declare_test_host_execution!(Reth, RpcBpo2);
// NOTE:
// - 1 EIP-8025 (in-block created-code resolution)
declare_test_host_execution!(Reth, RpcGlamsterdamDevnet7, failures = 1);
// NOTE:
// - 12 EIP-7610
//    - test_init_collision_create_tx
//    - test_init_collision_create_opcode
//    - test_collision_with_create2_revert_in_initcode
//    - test_create2_collision_storage
// - 1 EIP-8025 (in-block created-code resolution)
//    - test_witness_codes_create_same_hash_then_read
declare_test_host_execution!(Reth, EestGlamsterdamDevnet7, failures = 13);
