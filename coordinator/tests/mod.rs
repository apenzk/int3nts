//! Test module organization
//!
//! This module re-exports test helpers for use in test files.

mod helpers;
mod helpers_mock_server;

#[allow(unused_imports)]
pub use helpers::{
    build_test_config_with_evm, build_test_config_with_mock_server, build_test_config_with_mvm,
    build_test_config_with_svm,
    create_default_escrow_event, create_default_escrow_event_evm,
    create_default_fulfillment, create_default_mvm_transaction,
    create_default_intent_evm, create_default_intent_mvm, DUMMY_ESCROW_ID_MVM, DUMMY_EXPIRY,
    DUMMY_INTENT_ID, DUMMY_PUBLIC_KEY, DUMMY_REGISTERED_AT, DUMMY_REQUESTER_ADDR_EVM,
    DUMMY_REQUESTER_ADDR_HUB, DUMMY_REQUESTER_ADDR_MVMCON, DUMMY_REQUESTER_ADDR_SVM,
    DUMMY_SOLVER_ADDR_EVM, DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_ADDR_MVMCON, DUMMY_SOLVER_ADDR_SVM,
    DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_INTENT_ADDR_HUB, DUMMY_TOKEN_ADDR_EVM, DUMMY_TX_HASH, DUMMY_METADATA_ADDR_MVM,
    DUMMY_SVM_ESCROW_PROGRAM_ID,
    DUMMY_TOKEN_ADDR_FANTOM,
    DUMMY_INTENT_ID_FULL,
};

#[allow(unused_imports)]
pub use helpers_mock_server::{
    create_solver_registry_resource_with_mvm_address,
    create_solver_registry_resource_with_evm_address,
    create_solver_registry_resource_with_svm_address,
    setup_mock_server_with_error,
    setup_mock_server_with_mvm_address_response,
    setup_mock_server_with_evm_address_response,
    setup_mock_server_with_svm_address_response,
    setup_mock_server_with_registry_evm, setup_mock_server_with_registry_mvm,
    setup_mock_server_with_solver_registry, setup_mock_server_with_solver_registry_config,
};
