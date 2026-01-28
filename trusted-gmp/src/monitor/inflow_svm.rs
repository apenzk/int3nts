//! Inflow SVM-specific monitoring functions
//!
//! This module contains Solana-specific escrow polling logic
//! for escrow accounts on connected SVM chains.

use crate::config::Config;
use crate::monitor::generic::{ChainType, EscrowEvent};
use crate::svm_client::{pubkey_to_hex, SvmClient};
use anyhow::{Context, Result};

/// Polls the SVM connected chain for escrow accounts.
///
/// This function queries program accounts for the SVM escrow program and
/// converts each escrow account into the generic EscrowEvent format.
pub async fn poll_svm_escrow_events(config: &Config) -> Result<Vec<EscrowEvent>> {
    let connected_chain_svm = config
        .connected_chain_svm
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No connected SVM chain configured"))?;

    let client = SvmClient::new(
        &connected_chain_svm.rpc_url,
        &connected_chain_svm.escrow_program_id,
    )
    .context(format!(
        "Failed to create SVM client for RPC URL: {}",
        connected_chain_svm.rpc_url
    ))?;

    let escrows = client
        .get_all_escrows()
        .await
        .context("Failed to fetch SVM escrow accounts")?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let mut events = Vec::new();
    for escrow in escrows {
        let intent_id = format!("0x{}", hex::encode(escrow.escrow.intent_id));
        let escrow_id = pubkey_to_hex(&escrow.pubkey);
        let token_mint = pubkey_to_hex(&escrow.escrow.token_mint);
        let requester = pubkey_to_hex(&escrow.escrow.requester);
        let solver = pubkey_to_hex(&escrow.escrow.reserved_solver);
        let expiry = if escrow.escrow.expiry < 0 {
            0
        } else {
            escrow.escrow.expiry as u64
        };

        events.push(EscrowEvent {
            escrow_id,
            intent_id,
            offered_metadata: format!("{{\"inner\":\"{}\"}}", token_mint),
            offered_amount: escrow.escrow.amount,
            desired_metadata: "{}".to_string(),
            desired_amount: 0,
            revocable: false,
            requester_addr: requester,
            reserved_solver_addr: Some(solver),
            chain_id: connected_chain_svm.chain_id,
            chain_type: ChainType::Svm,
            expiry_time: expiry,
            timestamp,
        });
    }

    Ok(events)
}
