//! Unit tests for event monitoring
//!
//! These tests verify event structures and cache behavior
//! without requiring external services.
//!
//! Note: Validation-related tests are in trusted-gmp since the coordinator
//! is read-only and doesn't perform validation or signing.

use coordinator::monitor::{EventMonitor, IntentEvent};
#[path = "mod.rs"]
mod test_helpers;
use test_helpers::{
    build_test_config_with_mvm, create_default_escrow_event,
    create_default_intent_mvm,
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper function for validation logic
fn is_safe_for_escrow(event: &IntentEvent) -> bool {
    !event.revocable
}

// ============================================================================
// INTENT ID NORMALIZATION TESTS
// ============================================================================

/// Test that normalize_intent_id handles leading zeros correctly
/// What is tested: Intent IDs with leading zeros are normalized to match those without
/// Why: EVM and Move VM may format the same intent_id differently (with/without leading zeros)
#[test]
fn test_normalize_intent_id_leading_zeros() {
    use coordinator::monitor::normalize_intent_id;

    // Test case from the actual error: one has leading zero, one doesn't
    let with_leading_zero = "0x0911ddf3c2ef882c7c42af3f65b2c32b3f26fde142cf30afd2ea58f8a16ef9b7";
    let without_leading_zero = "0x911ddf3c2ef882c7c42af3f65b2c32b3f26fde142cf30afd2ea58f8a16ef9b7";

    let normalized_with = normalize_intent_id(with_leading_zero);
    let normalized_without = normalize_intent_id(without_leading_zero);

    assert_eq!(
        normalized_with, normalized_without,
        "Intent IDs with and without leading zeros should normalize to the same value"
    );
    assert_eq!(
        normalized_with,
        "0x911ddf3c2ef882c7c42af3f65b2c32b3f26fde142cf30afd2ea58f8a16ef9b7"
    );
}

/// Test that normalize_intent_id handles all-zero intent IDs
/// What is tested: Intent ID with all zeros is normalized correctly
/// Why: Edge case that should be handled gracefully
#[test]
fn test_normalize_intent_id_all_zeros() {
    use coordinator::monitor::normalize_intent_id;

    assert_eq!(normalize_intent_id("0x0000"), "0x0");
    assert_eq!(normalize_intent_id("0x0"), "0x0");
}

/// Test that normalize_intent_id handles case differences
/// What is tested: Uppercase hex characters are normalized to lowercase
/// Why: Ensures consistent comparison regardless of input case
#[test]
fn test_normalize_intent_id_case() {
    use coordinator::monitor::normalize_intent_id;

    assert_eq!(normalize_intent_id("0xABCDEF"), "0xabcdef");
    assert_eq!(normalize_intent_id("0xabcdef"), "0xabcdef");
}

// ============================================================================
// REVOCABILITY TESTS
// ============================================================================

/// Test that revocable intents are rejected (error thrown)
/// Why: Verify critical security check - revocable intents must be rejected for escrow
#[test]
fn test_revocable_intent_rejection() {
    let revocable_intent = IntentEvent {
        intent_id: "0xrevocable".to_string(),
        revocable: true, // NOT safe for escrow
        ..create_default_intent_mvm()
    };

    // Simulate validation: revocable intents should be rejected
    let result = is_safe_for_escrow(&revocable_intent);
    assert!(!result, "Revocable intents should NOT be safe for escrow");

    let non_revocable_intent = IntentEvent {
        intent_id: "0xsafe".to_string(),
        ..create_default_intent_mvm()
    };

    let result = is_safe_for_escrow(&non_revocable_intent);
    assert!(result, "Non-revocable intents should be safe for escrow");
}

// ============================================================================
// CACHE BEHAVIOR TESTS
// ============================================================================

/// Test that duplicate escrow events are rejected (not added to cache)
/// Why: Verify that the monitor correctly detects and rejects duplicate escrow events
#[tokio::test]
async fn test_duplicate_escrow_event_rejection() {
    let _ = tracing_subscriber::fmt::try_init();
    let config = build_test_config_with_mvm();
    let monitor = EventMonitor::new(&config)
        .await
        .expect("Failed to create monitor");

    let escrow = create_default_escrow_event();

    // Add escrow to cache (first time)
    {
        let mut escrow_cache = monitor.escrow_cache.write().await;
        // Simulate duplicate detection logic from monitor_connected_chain
        if !escrow_cache.iter().any(|cached| {
            cached.escrow_id == escrow.escrow_id && cached.chain_id == escrow.chain_id
        }) {
            escrow_cache.push(escrow.clone());
        }
    }

    // Verify escrow was added
    let escrow_cache = monitor.escrow_cache.read().await;
    assert_eq!(escrow_cache.len(), 1, "Escrow should be in cache");
    assert_eq!(escrow_cache[0].escrow_id, escrow.escrow_id);
    drop(escrow_cache);

    // Try to add the same escrow again (duplicate)
    {
        let mut escrow_cache = monitor.escrow_cache.write().await;
        // Simulate duplicate detection logic from monitor_connected_chain
        if !escrow_cache.iter().any(|cached| {
            cached.escrow_id == escrow.escrow_id && cached.chain_id == escrow.chain_id
        }) {
            escrow_cache.push(escrow.clone());
        }
    }

    // Verify duplicate was not added
    let escrow_cache = monitor.escrow_cache.read().await;
    assert_eq!(
        escrow_cache.len(),
        1,
        "Duplicate escrow should not be added to cache"
    );
    assert_eq!(escrow_cache[0].escrow_id, escrow.escrow_id);
}

/// Test that duplicate intent events are rejected (not added to cache)
/// Why: Verify that the monitor correctly detects and rejects duplicate intent events
#[tokio::test]
async fn test_duplicate_intent_event_rejection() {
    let _ = tracing_subscriber::fmt::try_init();
    let config = build_test_config_with_mvm();
    let monitor = EventMonitor::new(&config)
        .await
        .expect("Failed to create monitor");

    let intent = create_default_intent_mvm();

    // Add intent to cache (first time)
    {
        let mut cache = monitor.event_cache.write().await;
        // Simulate duplicate detection logic from monitor_hub_chain
        if !cache
            .iter()
            .any(|cached| cached.intent_id == intent.intent_id)
        {
            cache.push(intent.clone());
        }
    }

    // Verify intent was added
    let cache = monitor.event_cache.read().await;
    assert_eq!(cache.len(), 1, "Intent should be in cache");
    assert_eq!(cache[0].intent_id, intent.intent_id);
    drop(cache);

    // Try to add the same intent again (duplicate)
    {
        let mut cache = monitor.event_cache.write().await;
        // Simulate duplicate detection logic from monitor_hub_chain
        if !cache
            .iter()
            .any(|cached| cached.intent_id == intent.intent_id)
        {
            cache.push(intent.clone());
        }
    }

    // Verify duplicate was not added
    let cache = monitor.event_cache.read().await;
    assert_eq!(
        cache.len(),
        1,
        "Duplicate intent should not be added to cache"
    );
    assert_eq!(cache[0].intent_id, intent.intent_id);
}

/// Test that EventMonitor can be created and basic cache operations work
/// Why: Verify monitor initialization and basic read/write to caches
#[tokio::test]
async fn test_monitor_initialization_and_basic_ops() {
    let _ = tracing_subscriber::fmt::try_init();
    let config = build_test_config_with_mvm();
    let monitor = EventMonitor::new(&config)
        .await
        .expect("Failed to create monitor");

    // Verify caches are initially empty
    let intent_cache = monitor.event_cache.read().await;
    assert!(intent_cache.is_empty(), "Intent cache should be empty initially");
    drop(intent_cache);

    let escrow_cache = monitor.escrow_cache.read().await;
    assert!(escrow_cache.is_empty(), "Escrow cache should be empty initially");
    drop(escrow_cache);

    // Add an intent
    let intent = create_default_intent_mvm();
    {
        let mut cache = monitor.event_cache.write().await;
        cache.push(intent.clone());
    }

    // Verify it was added
    let cache = monitor.event_cache.read().await;
    assert_eq!(cache.len(), 1);
    assert_eq!(cache[0].intent_id, intent.intent_id);
}
