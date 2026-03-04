//! Draft-intent acceptance logic
//!
//! Determines whether the solver should sign a draftintent based on:
//! - Token pair validation (must be in configured supported pairs)
//! - Exchange rate validation (offered amount must meet required rate for the pair)

use std::collections::HashMap;

/// Token pair identifier for exchange rate lookup
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenPair {
    pub offered_chain_id: u64,
    pub offered_token: String,
    pub desired_chain_id: u64,
    pub desired_token: String,
}

/// Info about a token pair: exchange rate and fee parameters.
#[derive(Debug, Clone, Copy)]
pub struct TokenPairInfo {
    /// Exchange rate (how many offered tokens per 1 desired token)
    pub rate: f64,
    /// Fee in basis points (e.g., 50 = 0.5%, covers solver opportunity cost)
    pub fee_bps: u64,
    /// MOVE-to-offered-token conversion rate in base units.
    /// How many offered-token smallest units per 1 MOVE smallest unit (Octa).
    /// e.g., for USD tokens (6 decimals) with MOVE (8 decimals) at 1:1 price: 0.01
    pub move_rate: f64,
}

/// Acceptance config structure
pub struct AcceptanceConfig {
    /// Base fee in MOVE smallest units (covers solver gas costs).
    /// Single value across all token pairs.
    pub base_fee_in_move: u64,
    /// Supported token pairs with exchange rates and fee parameters
    /// Key: TokenPair (offered_chain_id, offered_token, desired_chain_id, desired_token)
    /// Value: Exchange rate and fee info
    pub token_pairs: HashMap<TokenPair, TokenPairInfo>,
}

/// Draft-intent data from coordinator API
#[derive(Debug, Clone)]
pub struct DraftintentData {
    pub intent_id: String,          // Intent ID (hex string)
    pub offered_token: String,      // Contract address
    pub offered_amount: u64,
    pub offered_chain_id: u64,
    pub desired_token: String,      // Contract address
    pub desired_amount: u64,
    pub desired_chain_id: u64,
    pub fee_in_offered_token: u64,            // Fee embedded in exchange rate (reduces desired_amount)
}

/// Result of acceptance evaluation
#[derive(Debug)]
pub enum AcceptanceResult {
    Accept,
    Reject(String),  // Reason for rejection
}

/// Calculate the required fee for a given offered amount and fee parameters.
///
/// Formula: `min_fee_offered + ceil(offered_amount * fee_bps / 10000)`
///
/// The `min_fee_offered` is the base_fee_in_move converted from MOVE to the offered token.
/// This conversion must be done by the caller before passing it here.
pub fn calculate_required_fee(offered_amount: u64, min_fee_offered: u64, fee_bps: u64) -> u64 {
    let bps_fee = if fee_bps > 0 {
        // ceil(offered_amount * fee_bps / 10000)
        let numerator = offered_amount as u128 * fee_bps as u128;
        ((numerator + 9999) / 10000) as u64
    } else {
        0
    };
    min_fee_offered.saturating_add(bps_fee)
}

/// Convert the solver's MOVE-denominated base_fee_in_move to the offered token.
///
/// Formula: `ceil(base_fee_in_move * move_rate)`
/// where `move_rate` is how many offered tokens per 1 MOVE (smallest units).
///
/// Returns the base fee in offered token smallest units.
pub fn convert_base_fee_in_move_to_offered(base_fee_in_move: u64, move_rate: f64) -> u64 {
    if base_fee_in_move == 0 {
        return 0;
    }
    (base_fee_in_move as f64 * move_rate).ceil() as u64
}

/// Evaluate whether to accept a draft intent
pub fn evaluate_draft_acceptance(draft: &DraftintentData, config: &AcceptanceConfig) -> AcceptanceResult {
    // Create token pair key for lookup
    let pair = TokenPair {
        offered_chain_id: draft.offered_chain_id,
        offered_token: draft.offered_token.clone(),
        desired_chain_id: draft.desired_chain_id,
        desired_token: draft.desired_token.clone(),
    };

    // Check if token pair is supported
    let info = match config.token_pairs.get(&pair) {
        Some(info) => *info,
        None => {
            return AcceptanceResult::Reject(format!(
                "Token pair not supported: {}:{} -> {}:{}",
                draft.offered_chain_id, draft.offered_token,
                draft.desired_chain_id, draft.desired_token
            ));
        }
    };

    // Calculate required offered amount based on exchange rate
    // exchange_rate = offered_tokens_per_desired_token
    // required_offered = desired_amount * exchange_rate
    let required_offered = (draft.desired_amount as f64 * info.rate) as u64;

    if draft.offered_amount < required_offered {
        return AcceptanceResult::Reject(format!(
            "Swap rejected: offered {} < required {} (rate: {} offered/desired)",
            draft.offered_amount, required_offered, info.rate
        ));
    }

    // Convert base_fee_in_move from MOVE to offered token using the pair's move_rate.
    // move_rate = offered-token-smallest-units per 1 Octa (MOVE smallest unit).
    let min_fee_offered = convert_base_fee_in_move_to_offered(config.base_fee_in_move, info.move_rate);

    // Validate fee_in_offered_token meets solver's minimum requirements
    let required_fee = calculate_required_fee(draft.offered_amount, min_fee_offered, info.fee_bps);
    if draft.fee_in_offered_token < required_fee {
        return AcceptanceResult::Reject(format!(
            "Fee rejected: fee_in_offered_token {} < required {} (base_fee_in_move: {} MOVE, min_fee_offered: {}, fee_bps: {})",
            draft.fee_in_offered_token, required_fee, config.base_fee_in_move, min_fee_offered, info.fee_bps
        ));
    }

    AcceptanceResult::Accept
}


