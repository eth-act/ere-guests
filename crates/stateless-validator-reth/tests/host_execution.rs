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
// - 2 EIP-2780 (authorization charges)
//    - test_auth_base_net_new_only
//    - test_multi_authorization_intra_tx_state
// - 1 EIP-7702 (delegation clearing)
//    - test_delegation_clearing_and_set
// - 1 EIP-8025 (in-block created-code resolution)
//    - test_witness_codes_create_same_hash_then_read
// - 1 EIP-8037 (state creation gas for set code)
//    - test_same_tx_clear_then_reset_pre_delegated
declare_test_host_execution!(Reth, EestGlamsterdamDevnet7, failures = 17);
