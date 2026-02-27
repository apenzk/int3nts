//! Unit tests for outflow fulfillment service
//!
//! These tests verify that the outflow service correctly handles outflow intent fulfillment,
//! including service initialization and basic functionality.

use solver::{
    service::tracker::IntentTracker,
    service::outflow::OutflowService,
    service::liquidity::LiquidityMonitor,
};
use std::sync::Arc;

#[path = "helpers.rs"]
mod test_helpers;
use test_helpers::create_default_solver_config;

/// Create a LiquidityMonitor for testing
fn create_test_liquidity_monitor(config: &solver::config::SolverConfig) -> Arc<LiquidityMonitor> {
    Arc::new(LiquidityMonitor::new(config.clone(), config.liquidity.clone()).unwrap())
}

// ============================================================================
// OUTFLOW SERVICE TESTS
// ============================================================================

/// What is tested: OutflowService::new() creates a service successfully
/// Why: Ensure service initialization works correctly
#[test]
fn test_outflow_service_new() {
    let config = create_default_solver_config();
    let tracker = Arc::new(IntentTracker::new(&config).unwrap());
    let monitor = create_test_liquidity_monitor(&config);
    let _service = OutflowService::new(config, tracker, monitor).unwrap();
}

/// What is tested: poll_and_execute_transfers() returns empty list when no pending outflow intents
/// Why: Ensure the service correctly handles the case when there are no intents to process
///
/// Note: Uses explicit Runtime::block_on to avoid nested runtime issues from reqwest::Client
#[test]
fn test_poll_and_execute_transfers_empty() {
    // Create runtime in advance, then pass it into the service creation
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let config = create_default_solver_config();

    // These create reqwest::Client which may internally use tokio runtime
    let tracker = Arc::new(IntentTracker::new(&config).unwrap());
    let monitor = create_test_liquidity_monitor(&config);
    let service = OutflowService::new(config, tracker, monitor).unwrap();

    let result = rt.block_on(service.poll_and_execute_transfers()).unwrap();
    assert_eq!(result.len(), 0);
}


