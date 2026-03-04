//! Unit tests for draft intent acceptance logic
//!
//! These tests verify that the solver correctly evaluates draft intents
//! based on token types and amounts.

use solver::acceptance::{AcceptanceConfig, AcceptanceResult, DraftintentData, TokenPairInfo, calculate_required_fee, convert_base_fee_in_move_to_offered, evaluate_draft_acceptance};
use std::collections::HashMap;

#[path = "helpers.rs"]
mod test_helpers;
use test_helpers::{
    create_default_token_pair, DUMMY_INTENT_ID, DUMMY_TOKEN_ADDR_HUB, DUMMY_TOKEN_ADDR_MVMCON,
    DUMMY_TOKEN_ADDR_UNSUPPORTED,
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a default acceptance config with test values
fn test_config() -> AcceptanceConfig {
    use solver::acceptance::TokenPair;

    let mut token_pairs = HashMap::new();

    // Token A -> Token B (1:1 rate, 0.5% fee)
    token_pairs.insert(
        create_default_token_pair(),
        TokenPairInfo { rate: 1.0, fee_bps: 50, move_rate: 1.0 },
    );

    // Token A -> Token C (chain 2) (0.5 rate: 1 Token C = 0.5 Token A, cross-chain, 0.5% fee)
    token_pairs.insert(
        TokenPair {
            desired_token: DUMMY_TOKEN_ADDR_UNSUPPORTED.to_string(), // Different token address on chain 2 to test multiple token pairs
            ..create_default_token_pair()
        },
        TokenPairInfo { rate: 0.5, fee_bps: 50, move_rate: 0.5 },
    );

    AcceptanceConfig {
        base_fee_in_move: 100,
        token_pairs,
    }
}

/// Create a default draft intent data with test values
/// This can be customized using Rust's struct update syntax:
/// ```
/// let draft = create_default_draft_data();
/// let custom_draft = DraftintentData {
///     offered_amount: 500000,
///     desired_amount: 1000000,
///     ..draft
/// };
/// ```
fn create_default_draft_data() -> DraftintentData {
    DraftintentData {
        intent_id: DUMMY_INTENT_ID.to_string(),
        offered_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        offered_amount: 1000000,
        offered_chain_id: 1,
        desired_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
        desired_amount: 1000000,
        desired_chain_id: 2,
        fee_in_offered_token: 6000,
    }
}

/// Test that token pair swaps are accepted when offered >= required amount at configured exchange rate
/// What is tested: Token pair validation and exchange rate calculation (1:1 rate in this test)
/// Why: Solver should accept swaps when offered amount meets the configured exchange rate for the token pair
#[test]
fn test_token_pair_accept() {
    let config = test_config();
    let draft = create_default_draft_data(); // 1:1 rate, offered=1000000, desired=1000000
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Accept));
}

/// Test that token pair swaps are rejected when offered < required amount at configured exchange rate
/// What is tested: Exchange rate validation (1:1 rate in this test)
/// Why: Solver should reject swaps when offered amount doesn't meet the configured exchange rate for the token pair
#[test]
fn test_token_pair_reject_unfavorable() {
    let config = test_config();
    let draft = DraftintentData {
        offered_amount: 500000,  // 0.5 is less than the required amount 1.0 at configured 1:1 exchange rate
        desired_amount: 1000000,  // 1.0 requires 1.0 offered at configured 1:1 exchange rate
        ..create_default_draft_data()
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Reject(_)));
}

/// Test that token pair swaps with non-1:1 exchange rates are accepted when offered meets configured rate
/// What is tested: Exchange rate calculation for configured token pairs (0.5 rate in this test)
/// Why: Solver should accept swaps when offered amount meets the configured exchange rate for the token pair
#[test]
fn test_token_pair_with_exchange_rate_accept() {
    let config = test_config();
    let draft = DraftintentData {
        desired_token: DUMMY_TOKEN_ADDR_UNSUPPORTED.to_string(), // Token address from configured pair (Token A -> Token C)
        desired_amount: 2000000,  // 2.0 Token C (at 0.5 rate, requires 1.0 offered)
        ..create_default_draft_data()  // offered_amount: 1000000 (1.0) meets the requirement (2.0 * 0.5 = 1.0)
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Accept));
}

/// Test that unsupported token pairs are rejected
/// What is tested: Token pair validation
/// Why: Solver should only accept configured token pairs
#[test]
fn test_unsupported_token_pair_rejected() {
    let config = test_config();
    let draft = DraftintentData {
        offered_token: DUMMY_TOKEN_ADDR_UNSUPPORTED.to_string(), // Unsupported token (not in any configured pair)
        ..create_default_draft_data()  // offered_amount: 1000000, desired_amount: 1000000, but pair is not configured
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Reject(_)));
}

// ============================================================================
// FEE TESTS
// ============================================================================

/// Create a config with fees enabled: base_fee_in_move=1000 MOVE, fee_bps=50 (0.5%)
/// With rate=1.0, base_fee_in_move converts to 1000 in offered token (ceil(1000 * 1.0) = 1000)
fn test_config_with_fees() -> AcceptanceConfig {
    let mut token_pairs = HashMap::new();
    token_pairs.insert(
        create_default_token_pair(),
        TokenPairInfo { rate: 1.0, fee_bps: 50, move_rate: 1.0 },
    );
    AcceptanceConfig { base_fee_in_move: 1000, token_pairs }
}

/// Test that convert_base_fee_in_move_to_offered correctly converts MOVE base_fee_in_move to offered token
/// What is tested: MOVE → offered token conversion
/// Why: base_fee_in_move is denominated in MOVE; protocol must convert before fee validation
#[test]
fn test_convert_base_fee_in_move_to_offered() {
    // 1:1 rate — no change
    assert_eq!(convert_base_fee_in_move_to_offered(1000, 1.0), 1000);
    // 0.5 rate — ceil(1000 * 0.5) = 500
    assert_eq!(convert_base_fee_in_move_to_offered(1000, 0.5), 500);
    // 2.0 rate — ceil(1000 * 2.0) = 2000
    assert_eq!(convert_base_fee_in_move_to_offered(1000, 2.0), 2000);
    // Fractional result — ceil(100 * 0.3) = ceil(30.0) = 30
    assert_eq!(convert_base_fee_in_move_to_offered(100, 0.3), 30);
    // Ceil rounding — ceil(100 * 0.33) = ceil(33.0) = 33
    assert_eq!(convert_base_fee_in_move_to_offered(100, 0.33), 33);
    // Zero base_fee_in_move
    assert_eq!(convert_base_fee_in_move_to_offered(0, 1.0), 0);
}

/// Test that calculate_required_fee computes correctly: min_fee_offered + ceil(amount * bps / 10000)
/// What is tested: Fee calculation formula
/// Why: Ensure the fee formula matches the documented specification
#[test]
fn test_calculate_required_fee() {
    // 1000 + ceil(1000000 * 50 / 10000) = 1000 + 5000 = 6000
    assert_eq!(calculate_required_fee(1000000, 1000, 50), 6000);
    // Zero fees
    assert_eq!(calculate_required_fee(1000000, 0, 0), 0);
    // Only min_fee
    assert_eq!(calculate_required_fee(1000000, 500, 0), 500);
    // Only bps fee: ceil(100 * 50 / 10000) = ceil(0.5) = 1
    assert_eq!(calculate_required_fee(100, 0, 50), 1);
    // Exact division: 10000 * 50 / 10000 = 50
    assert_eq!(calculate_required_fee(10000, 0, 50), 50);
}

/// Test that draft with sufficient fee_in_offered_token is accepted
/// What is tested: Fee validation in acceptance logic
/// Why: Solver should accept drafts where fee_in_offered_token >= required fee
#[test]
fn test_fee_sufficient_accepted() {
    let config = test_config_with_fees();
    let draft = DraftintentData {
        fee_in_offered_token: 6000, // Exactly the required fee: 1000 + ceil(1000000 * 50 / 10000)
        ..create_default_draft_data()
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Accept));
}

/// Test that draft with excess fee_in_offered_token is accepted
/// What is tested: Fee validation allows overpayment
/// Why: Users may pay more than the minimum required fee
#[test]
fn test_fee_excess_accepted() {
    let config = test_config_with_fees();
    let draft = DraftintentData {
        fee_in_offered_token: 10000, // More than required 6000
        ..create_default_draft_data()
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Accept));
}

/// Test that draft with insufficient fee_in_offered_token is rejected
/// What is tested: Fee validation rejects underpayment
/// Why: Solver should reject drafts where fee_in_offered_token < required fee
#[test]
fn test_fee_insufficient_rejected() {
    let config = test_config_with_fees();
    let draft = DraftintentData {
        fee_in_offered_token: 5999, // One less than required 6000
        ..create_default_draft_data()
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Reject(_)));
}

/// Test that draft with zero fee_in_offered_token is rejected when fees are configured
/// What is tested: Fee validation rejects zero fee when min_fee > 0
/// Why: Solver must not accept free trades when fees are configured
#[test]
fn test_fee_zero_rejected_when_configured() {
    let config = test_config_with_fees();
    let draft = DraftintentData {
        fee_in_offered_token: 0, // No fee offered but 6000 required
        ..create_default_draft_data()
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Reject(_)));
}

/// Test that zero fee_in_offered_token is accepted when no fees are configured
/// What is tested: Fee validation is a no-op when fees are zero
/// Why: Backward compatibility - solvers with no fee config should accept zero-fee drafts
#[test]
fn test_fee_zero_accepted_when_no_fees() {
    let mut token_pairs = HashMap::new();
    token_pairs.insert(
        create_default_token_pair(),
        TokenPairInfo { rate: 1.0, fee_bps: 0, move_rate: 1.0 },
    );
    let config = AcceptanceConfig {
        base_fee_in_move: 0,
        token_pairs,
    };
    let draft = DraftintentData {
        fee_in_offered_token: 0,
        ..create_default_draft_data()
    };
    assert!(matches!(evaluate_draft_acceptance(&draft, &config), AcceptanceResult::Accept));
}


