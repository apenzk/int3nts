//! Test module organization
//!
//! This module re-exports test helpers for use in test files.

mod helpers;

#[allow(unused_imports)]
pub use helpers::{
    build_test_config_with_evm, build_test_config_with_mvm, build_test_config_with_svm,
    DUMMY_APPROVER_EVM_PUBKEY_HASH, DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_INTENT_ID,
    DUMMY_PUBLIC_KEY, DUMMY_REGISTERED_AT, DUMMY_SOLVER_ADDR_EVM, DUMMY_SOLVER_ADDR_HUB,
    DUMMY_SOLVER_ADDR_MVMCON, DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SVM_ESCROW_PROGRAM_ID,
    DUMMY_TX_HASH, TEST_MVM_CHAIN_ID, TEST_SVM_CHAIN_ID,
};
