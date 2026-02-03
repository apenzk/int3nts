//! MVM-specific test suite
//!
//! This module includes all MVM-specific tests from the mvm/ subdirectory.
//!
//! Test files:
//! - hub_client_tests.rs: Hub chain client tests (hub is always MVM)
//! - chain_client_tests.rs: Connected MVM chain client tests (synchronized with EVM/SVM)

#[path = "mvm/hub_client_tests.rs"]
mod hub_client_tests;

#[path = "mvm/chain_client_tests.rs"]
mod chain_client_tests;
