//! GMP-specific SVM Client
//!
//! Wraps the shared `chain_clients_svm::SvmClient` and adds GMP-specific methods
//! for reading outbound nonce counters and message accounts from the GMP program.

use anyhow::{Context, Result};
use chain_clients_svm::SvmClient;
use solana_sdk::pubkey::Pubkey;

// ============================================================================
// CLIENT
// ============================================================================

pub struct GmpSvmClient {
    svm_client: SvmClient,
}

impl GmpSvmClient {
    pub fn new(rpc_url: &str, program_id: &str) -> Result<Self> {
        let svm_client =
            SvmClient::new(rpc_url, program_id).context("Failed to create shared SVM client")?;
        Ok(Self { svm_client })
    }

    /// Read the global outbound nonce from the GMP program.
    /// PDA seeds: ["nonce_out"]
    /// Returns the nonce value (next nonce to be assigned), or 0 if the account doesn't exist.
    pub async fn get_outbound_nonce(
        &self,
        gmp_program_id: &Pubkey,
    ) -> Result<u64> {
        let gmp_program_id = to_solana_program_pubkey(gmp_program_id);
        let (nonce_pda, _) =
            chain_clients_svm::solana_program::pubkey::Pubkey::find_program_address(
                &[b"nonce_out"],
                &gmp_program_id,
            );

        let data = self.svm_client.get_raw_account_data(&nonce_pda).await?;
        let Some(data) = data else {
            return Ok(0); // No nonce account = no messages sent yet
        };

        // OutboundNonceAccount layout: disc(1) + nonce(8) + bump(1) = 10 bytes
        if data.len() < 9 {
            anyhow::bail!("OutboundNonceAccount too short: {} bytes", data.len());
        }

        let nonce = u64::from_le_bytes(
            data[1..9]
                .try_into()
                .context("Failed to parse nonce bytes")?,
        );
        Ok(nonce)
    }

    /// Read a stored outbound message from the GMP program.
    /// PDA seeds: ["message", nonce.to_le_bytes()]
    /// Returns the parsed message, or None if the account doesn't exist.
    pub async fn get_message_data(
        &self,
        gmp_program_id: &Pubkey,
        nonce: u64,
    ) -> Result<Option<SvmOutboundMessage>> {
        let nonce_bytes = nonce.to_le_bytes();
        let gmp_program_id = to_solana_program_pubkey(gmp_program_id);
        let (message_pda, _) =
            chain_clients_svm::solana_program::pubkey::Pubkey::find_program_address(
                &[b"message", &nonce_bytes],
                &gmp_program_id,
            );

        let data = self.svm_client.get_raw_account_data(&message_pda).await?;
        let Some(data) = data else {
            return Ok(None);
        };

        // MessageAccount layout (Borsh):
        //   disc(1) + src_chain_id(4) + dst_chain_id(4) + nonce(8) +
        //   dst_addr(32) + remote_gmp_endpoint_addr(32) + payload_len(4) + payload(N) + bump(1)
        if data.len() < 86 {
            anyhow::bail!("MessageAccount too short: {} bytes", data.len());
        }

        let disc = data[0];
        if disc != 7 {
            anyhow::bail!(
                "MessageAccount discriminator mismatch: expected 7, got {}",
                disc
            );
        }

        let src_chain_id =
            u32::from_le_bytes(data[1..5].try_into().context("src_chain_id")?);
        let dst_chain_id =
            u32::from_le_bytes(data[5..9].try_into().context("dst_chain_id")?);
        let msg_nonce = u64::from_le_bytes(data[9..17].try_into().context("nonce")?);

        let mut dst_addr = [0u8; 32];
        dst_addr.copy_from_slice(&data[17..49]);

        let mut remote_gmp_endpoint_addr = [0u8; 32];
        remote_gmp_endpoint_addr.copy_from_slice(&data[49..81]);

        let payload_len =
            u32::from_le_bytes(data[81..85].try_into().context("payload_len")?) as usize;
        if data.len() < 85 + payload_len {
            anyhow::bail!(
                "MessageAccount payload truncated: need {} bytes, have {}",
                85 + payload_len,
                data.len()
            );
        }
        let payload = data[85..85 + payload_len].to_vec();

        Ok(Some(SvmOutboundMessage {
            src_chain_id,
            dst_chain_id,
            nonce: msg_nonce,
            dst_addr,
            remote_gmp_endpoint_addr,
            payload,
        }))
    }
}

// ============================================================================
// TYPES
// ============================================================================

/// Parsed SVM outbound message from on-chain MessageAccount.
#[derive(Debug, Clone)]
pub struct SvmOutboundMessage {
    pub src_chain_id: u32,
    pub dst_chain_id: u32,
    pub nonce: u64,
    pub dst_addr: [u8; 32],
    pub remote_gmp_endpoint_addr: [u8; 32],
    pub payload: Vec<u8>,
}

// ============================================================================
// HELPERS
// ============================================================================

/// Convert solana_sdk::Pubkey to solana_program::Pubkey (same bytes, different crate types).
fn to_solana_program_pubkey(
    pubkey: &Pubkey,
) -> chain_clients_svm::solana_program::pubkey::Pubkey {
    chain_clients_svm::solana_program::pubkey::Pubkey::new_from_array(pubkey.to_bytes())
}
