//! Solver HTTP API
//!
//! Provides acceptance ratio lookup for the verifier.

use crate::config::SolverConfig;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::net::{IpAddr, Ipv4Addr};
use warp::Filter;

#[derive(Debug)]
struct QueryError(String);

impl warp::reject::Reject for QueryError {}

/// Standard API response wrapper.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Response structure for exchange rate query.
#[derive(Debug, Serialize)]
pub struct ExchangeRateResponse {
    pub desired_token: String,
    pub desired_chain_id: u64,
    pub exchange_rate: f64,
}

/// Start the solver acceptance API server.
///
/// Exposes `GET /acceptance` for live ratio lookups by the verifier.
///
/// # Arguments
///
/// * `config` - Shared solver configuration
/// * `host` - Bind host for the acceptance API
/// * `port` - Bind port for the acceptance API
///
/// # Returns
///
/// - `()` - Runs until the process is stopped
pub async fn run_acceptance_server(config: Arc<SolverConfig>, host: String, port: u16) {
    let config_filter = warp::any().map(move || config.clone());

    let acceptance = warp::path("acceptance")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .and(config_filter)
        .and_then(get_exchange_rate_handler);

    // Normalize errors into JSON for callers.
    let routes = acceptance.recover(handle_rejection);
    // Fall back to loopback if host parsing fails.
    let ip: IpAddr = host
        .parse()
        .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    warp::serve(routes).run((ip, port)).await;
}

/// Handle `/acceptance` ratio queries from the verifier.
///
/// Returns the exchange rate for a specific token pair, or the first
/// match for a given offered token when no desired token is supplied.
///
/// # Arguments
///
/// * `params` - Raw query parameters from the request
/// * `config` - Shared solver configuration
///
/// # Returns
///
/// - `Ok(reply)` - Exchange rate response
/// - `Err(rejection)` - Missing/invalid query or unknown pair
async fn get_exchange_rate_handler(
    params: HashMap<String, String>,
    config: Arc<SolverConfig>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let offered_chain_id = params.get("offered_chain_id")
        .ok_or_else(|| warp::reject::custom(QueryError("Missing offered_chain_id parameter".to_string())))?;
    let offered_token = params.get("offered_token")
        .ok_or_else(|| warp::reject::custom(QueryError("Missing offered_token parameter".to_string())))?;

    let desired_chain_id = params.get("desired_chain_id");
    let desired_token = params.get("desired_token");

    let token_pairs = config
        .get_token_pairs()
        .map_err(|e| warp::reject::custom(QueryError(e.to_string())))?;

    // Find matching pair
    let (pair, rate) = if let (Some(d_chain_id), Some(d_token)) = (desired_chain_id, desired_token) {
        let offered_chain_id = offered_chain_id.parse::<u64>()
            .map_err(|e| warp::reject::custom(QueryError(format!("Invalid offered_chain_id: {}", e))))?;
        let desired_chain_id = d_chain_id.parse::<u64>()
            .map_err(|e| warp::reject::custom(QueryError(format!("Invalid desired_chain_id: {}", e))))?;

        let key = crate::acceptance::TokenPair {
            offered_chain_id,
            offered_token: offered_token.to_string(),
            desired_chain_id,
            desired_token: d_token.to_string(),
        };
        let rate = token_pairs.get(&key).copied().ok_or_else(|| {
            warp::reject::custom(QueryError(format!(
                "No exchange rate found for token pair: {}:{} -> {}:{}",
                offered_chain_id, offered_token, desired_chain_id, d_token
            )))
        })?;
        (key, rate)
    } else {
        // Find first matching pair by offered chain/token
        let offered_chain_id = offered_chain_id.parse::<u64>()
            .map_err(|e| warp::reject::custom(QueryError(format!("Invalid offered_chain_id: {}", e))))?;
        let (pair, rate) = token_pairs.iter()
            .find(|(pair, _)| {
                pair.offered_chain_id == offered_chain_id
                    && pair.offered_token == *offered_token
            })
            .map(|(pair, rate)| (pair.clone(), *rate))
            .ok_or_else(|| {
                warp::reject::custom(QueryError(format!(
                    "No exchange rate found for offered token {} on chain {}",
                    offered_token, offered_chain_id
                )))
            })?;
        (pair, rate)
    };

    Ok(warp::reply::json(&ApiResponse::<ExchangeRateResponse> {
        success: true,
        data: Some(ExchangeRateResponse {
            desired_token: pair.desired_token,
            desired_chain_id: pair.desired_chain_id,
            exchange_rate: rate,
        }),
        error: None,
    }))
}

/// Normalize rejections into a consistent JSON error response.
///
/// # Arguments
///
/// * `err` - Warp rejection
///
/// # Returns
///
/// - `Ok(reply)` - JSON error response with status
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    let message = if let Some(QueryError(msg)) = err.find::<QueryError>() {
        msg.clone()
    } else {
        "Internal server error".to_string()
    };

    let response = ApiResponse::<()> {
        success: false,
        data: None,
        error: Some(message),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::BAD_REQUEST,
    ))
}
