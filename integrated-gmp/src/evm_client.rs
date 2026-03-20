//! GMP-specific EVM Client
//!
//! Wraps the shared `chain_clients_evm::EvmClient` and adds GMP-specific methods
//! for message delivery, event polling, and relay authorization checks.

use anyhow::{Context, Result};
use chain_clients_evm::{EvmClient, EvmLog};
use sha3::{Digest, Keccak256};
use std::time::Duration;
use tracing::warn;

use crate::crypto::CryptoService;
use crate::integrated_gmp_relay::GmpMessage;

// ============================================================================
// CLIENT
// ============================================================================

pub struct GmpEvmClient {
    evm_client: EvmClient,
    gmp_endpoint_addr: String,
    chain_id: u32,
    relay_address: String,
}

impl GmpEvmClient {
    pub fn new(
        rpc_url: &str,
        gmp_endpoint_addr: &str,
        chain_id: u32,
        relay_address: &str,
    ) -> Result<Self> {
        let evm_client =
            EvmClient::new_rpc_only(rpc_url).context("Failed to create EVM RPC client")?;
        Ok(Self {
            evm_client,
            gmp_endpoint_addr: gmp_endpoint_addr.to_string(),
            chain_id,
            relay_address: relay_address.to_string(),
        })
    }

    pub fn chain_id(&self) -> u32 {
        self.chain_id
    }

    pub fn gmp_endpoint_addr(&self) -> &str {
        &self.gmp_endpoint_addr
    }

    pub fn relay_address(&self) -> &str {
        &self.relay_address
    }

    // ========================================================================
    // Block number
    // ========================================================================

    pub async fn get_block_number(&self) -> Result<u64> {
        self.evm_client.get_block_number().await
    }

    // ========================================================================
    // Authorization check
    // ========================================================================

    /// Check if a relay address is authorized on the GMP endpoint contract.
    pub async fn is_relay_authorized(&self, relay_addr: &str) -> Result<bool> {
        let selector = &Keccak256::digest(b"isRelayAuthorized(address)")[..4];

        let addr_clean = relay_addr.strip_prefix("0x").unwrap_or(relay_addr);
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(selector);
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(
            &hex::decode(addr_clean).context("Invalid EVM address hex")?,
        );

        let data_hex = format!("0x{}", hex::encode(&calldata));
        let result: String = self
            .evm_client
            .eth_call(&self.gmp_endpoint_addr, &data_hex)
            .await
            .context("Failed to check relay authorization on EVM")?;

        let clean = result.strip_prefix("0x").unwrap_or(&result);
        Ok(clean.ends_with('1'))
    }

    // ========================================================================
    // Message delivery check
    // ========================================================================

    /// Check if a message was already delivered on the EVM GMP endpoint.
    pub async fn is_message_delivered(
        &self,
        intent_id: &[u8],
        msg_type: u8,
    ) -> Result<bool> {
        let selector =
            &Keccak256::digest(b"isMessageDelivered(bytes32,uint8)")[..4];

        let mut calldata = Vec::with_capacity(68);
        calldata.extend_from_slice(selector);

        let mut intent_id_padded = [0u8; 32];
        let len = intent_id.len().min(32);
        intent_id_padded[..len].copy_from_slice(&intent_id[..len]);
        calldata.extend_from_slice(&intent_id_padded);

        let mut msg_type_padded = [0u8; 32];
        msg_type_padded[31] = msg_type;
        calldata.extend_from_slice(&msg_type_padded);

        let data_hex = format!("0x{}", hex::encode(&calldata));
        let result: String = self
            .evm_client
            .eth_call(&self.gmp_endpoint_addr, &data_hex)
            .await
            .context("Failed to call isMessageDelivered on EVM")?;

        let clean = result.strip_prefix("0x").unwrap_or(&result);
        Ok(clean.ends_with('1'))
    }

    // ========================================================================
    // Message delivery
    // ========================================================================

    /// Deliver a GMP message to this EVM chain.
    ///
    /// ABI-encodes `deliverMessage(uint32,bytes32,bytes)`, builds a legacy
    /// transaction, signs it with the relay's ECDSA key, and broadcasts.
    /// Returns the transaction hash.
    pub async fn deliver_message(
        &self,
        src_chain_id: u32,
        remote_gmp_endpoint_addr: &str,
        payload: &str,
        crypto_service: &CryptoService,
    ) -> Result<String> {
        let calldata =
            evm_encode_deliver_message(src_chain_id, remote_gmp_endpoint_addr, payload)?;

        self.send_signed_transaction(&calldata, crypto_service)
            .await
    }

    /// Wait for an EVM transaction receipt and verify success.
    pub async fn wait_for_receipt(&self, tx_hash: &str) -> Result<()> {
        for _ in 0..30 {
            let receipt: Option<serde_json::Value> =
                self.evm_client.get_transaction_receipt(tx_hash).await?;

            if let Some(receipt) = receipt {
                let status = receipt
                    .get("status")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow::anyhow!(
                        "EVM receipt for {} missing status field: {receipt}",
                        tx_hash
                    ))?;
                if status == "0x1" {
                    return Ok(());
                } else {
                    anyhow::bail!(
                        "EVM transaction {} failed with status: {}",
                        tx_hash,
                        status
                    );
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        anyhow::bail!(
            "Timed out waiting for EVM transaction receipt: {}",
            tx_hash
        )
    }

    // ========================================================================
    // Event polling
    // ========================================================================

    /// Poll MessageSent events from the GMP endpoint contract.
    pub async fn poll_message_sent_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<GmpMessage>> {
        let event_signature =
            evm_event_topic("MessageSent(uint32,bytes32,bytes,uint64)");

        let filter = serde_json::json!({
            "address": self.gmp_endpoint_addr,
            "topics": [event_signature],
            "fromBlock": format!("0x{:x}", from_block),
            "toBlock": format!("0x{:x}", to_block),
        });

        let logs: Vec<EvmLog> = self.evm_client.get_logs(filter).await?;

        let mut messages = Vec::new();
        for log in &logs {
            if let Some(msg) = self.parse_message_sent(log) {
                messages.push(msg);
            }
        }

        Ok(messages)
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Build, sign, and send a legacy EVM transaction.
    async fn send_signed_transaction(
        &self,
        calldata: &str,
        crypto_service: &CryptoService,
    ) -> Result<String> {
        let nonce = self
            .evm_client
            .get_transaction_count(&self.relay_address)
            .await
            .context("eth_getTransactionCount failed")?;

        let gas_price = self
            .evm_client
            .gas_price()
            .await
            .context("eth_gasPrice failed")?;

        let gas_limit: u64 = 2_000_000;

        let to_hex = self
            .gmp_endpoint_addr
            .strip_prefix("0x")
            .unwrap_or(&self.gmp_endpoint_addr);
        let to_bytes =
            hex::decode(to_hex).context("Failed to decode EVM 'to' address")?;

        let calldata_hex = calldata.strip_prefix("0x").unwrap_or(calldata);
        let data_bytes =
            hex::decode(calldata_hex).context("Failed to decode EVM calldata")?;

        // RLP-encode unsigned tx for EIP-155 signing:
        //   [nonce, gasPrice, gasLimit, to, value, data, chainId, 0, 0]
        let unsigned_items: Vec<Vec<u8>> = vec![
            rlp_encode_u64(nonce),
            rlp_encode_u64(gas_price),
            rlp_encode_u64(gas_limit),
            to_bytes.clone(),
            vec![], // value = 0
            data_bytes.clone(),
            rlp_encode_u64(self.chain_id as u64),
            vec![], // 0
            vec![], // 0
        ];
        let unsigned_rlp = rlp_encode_list(&unsigned_items);

        // Keccak256 hash
        let mut hasher = Keccak256::new();
        hasher.update(&unsigned_rlp);
        let tx_hash: [u8; 32] = hasher.finalize().into();

        // Sign with ECDSA key
        let (r, s, recovery_id) = crypto_service
            .sign_evm_transaction_hash(&tx_hash)
            .context("Failed to sign EVM transaction")?;

        // Compute EIP-155 v value: recovery_id + chainId * 2 + 35
        let v = (recovery_id as u64) + (self.chain_id as u64) * 2 + 35;

        // Build signed tx RLP: [nonce, gasPrice, gasLimit, to, value, data, v, r, s]
        // r and s are 32-byte big-endian integers; strip leading zeros for valid RLP.
        let r_trimmed = strip_leading_zeros(&r);
        let s_trimmed = strip_leading_zeros(&s);
        let signed_items: Vec<Vec<u8>> = vec![
            rlp_encode_u64(nonce),
            rlp_encode_u64(gas_price),
            rlp_encode_u64(gas_limit),
            to_bytes,
            vec![], // value = 0
            data_bytes,
            rlp_encode_u64(v),
            r_trimmed,
            s_trimmed,
        ];
        let signed_rlp = rlp_encode_list(&signed_items);
        let raw_tx = format!("0x{}", hex::encode(&signed_rlp));

        self.evm_client
            .send_raw_transaction(&raw_tx)
            .await
            .context("eth_sendRawTransaction failed")
    }

    /// Parse an EVM MessageSent log into a GmpMessage.
    fn parse_message_sent(&self, log: &EvmLog) -> Option<GmpMessage> {
        if log.topics.len() < 2 {
            return None;
        }

        // topics[1] = dstChainId (uint32, padded to 32 bytes)
        let dst_chain_id_hex = log.topics[1]
            .strip_prefix("0x")
            .unwrap_or(&log.topics[1]);
        let dst_chain_id =
            match u32::from_str_radix(dst_chain_id_hex.trim_start_matches('0'), 16) {
                Ok(id) => id,
                Err(e) => {
                    warn!(
                        "Failed to parse EVM MessageSent dstChainId hex '{}': {}",
                        dst_chain_id_hex, e
                    );
                    return None;
                }
            };

        // Parse non-indexed data: (bytes32 dstAddr, bytes payload, uint64 nonce)
        let data = log.data.strip_prefix("0x").unwrap_or(&log.data);

        // Minimum: dstAddr(64) + payloadOffset(64) + nonce(64) + payloadLen(64) = 256 hex chars
        if data.len() < 256 {
            warn!(
                "EVM MessageSent data too short: {} hex chars",
                data.len()
            );
            return None;
        }

        // Word 0 (0..64): dstAddr (bytes32)
        let dst_addr = format!("0x{}", &data[0..64]);

        // Word 1 (64..128): offset to payload data
        let payload_offset_hex = &data[64..128];
        let payload_offset = match usize::from_str_radix(
            payload_offset_hex.trim_start_matches('0'),
            16,
        ) {
            Ok(offset) => offset,
            Err(e) => {
                warn!(
                    "Failed to parse EVM MessageSent payload offset hex '{}': {}",
                    payload_offset_hex, e
                );
                return None;
            }
        };

        // Word 2 (128..192): nonce (uint64)
        let nonce_hex = &data[128..192];
        let nonce = match u64::from_str_radix(nonce_hex.trim_start_matches('0'), 16) {
            Ok(n) => n,
            Err(e) => {
                warn!(
                    "Failed to parse EVM MessageSent nonce hex '{}': {}",
                    nonce_hex, e
                );
                return None;
            }
        };

        // Payload at offset (in bytes, so offset*2 in hex chars)
        let payload_start = payload_offset * 2;
        if data.len() < payload_start + 64 {
            warn!(
                "EVM MessageSent data too short for payload at offset {}",
                payload_offset
            );
            return None;
        }

        // Payload length
        let payload_len_hex = &data[payload_start..payload_start + 64];
        let payload_len = match usize::from_str_radix(
            payload_len_hex.trim_start_matches('0'),
            16,
        ) {
            Ok(len) => len,
            Err(e) => {
                warn!(
                    "Failed to parse EVM MessageSent payload length hex '{}': {}",
                    payload_len_hex, e
                );
                return None;
            }
        };

        // Payload data
        let payload_data_start = payload_start + 64;
        let payload_data_end = payload_data_start + payload_len * 2;
        let payload = if payload_len > 0 && data.len() >= payload_data_end {
            format!("0x{}", &data[payload_data_start..payload_data_end])
        } else {
            "0x".to_string()
        };

        // Source address: GMP endpoint contract padded to 32 bytes
        let clean = self
            .gmp_endpoint_addr
            .strip_prefix("0x")
            .unwrap_or(&self.gmp_endpoint_addr)
            .to_lowercase();
        let gmp_addr = format!("0x{:0>64}", clean);

        Some(GmpMessage {
            src_chain_id: self.chain_id,
            remote_gmp_endpoint_addr: gmp_addr,
            dst_chain_id,
            dst_addr,
            payload,
            nonce,
        })
    }
}

// ============================================================================
// FREE FUNCTIONS (moved from integrated_gmp_relay.rs)
// ============================================================================

/// Compute the Keccak256 topic hash for an EVM event signature.
fn evm_event_topic(signature: &str) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(signature.as_bytes());
    format!("0x{}", hex::encode(hasher.finalize()))
}

/// ABI-encode a call to `deliverMessage(uint32,bytes32,bytes)`.
fn evm_encode_deliver_message(
    src_chain_id: u32,
    remote_gmp_endpoint_addr: &str,
    payload: &str,
) -> Result<String> {
    let mut hasher = Keccak256::new();
    hasher.update(b"deliverMessage(uint32,bytes32,bytes)");
    let hash = hasher.finalize();
    let selector = &hash[..4];

    let remote_gmp_endpoint_addr_bytes =
        parse_32_byte_address(remote_gmp_endpoint_addr)?;

    let payload_bytes = hex_to_bytes(payload)?;

    let mut data = Vec::new();

    // Selector
    data.extend_from_slice(selector);

    // Word 0: srcChainId
    let mut word = [0u8; 32];
    word[28..32].copy_from_slice(&src_chain_id.to_be_bytes());
    data.extend_from_slice(&word);

    // Word 1: remoteGmpEndpointAddr
    data.extend_from_slice(&remote_gmp_endpoint_addr_bytes);

    // Word 2: offset to payload (96 = 0x60, after 3 head words)
    let mut word = [0u8; 32];
    word[31] = 96;
    data.extend_from_slice(&word);

    // Dynamic section: payload length
    let mut word = [0u8; 32];
    let payload_len = payload_bytes.len() as u64;
    word[24..32].copy_from_slice(&payload_len.to_be_bytes());
    data.extend_from_slice(&word);

    // Payload data (right-padded to 32-byte boundary)
    data.extend_from_slice(&payload_bytes);
    let padding = (32 - (payload_bytes.len() % 32)) % 32;
    data.extend(std::iter::repeat(0u8).take(padding));

    Ok(format!("0x{}", hex::encode(data)))
}

fn parse_32_byte_address(addr: &str) -> Result<[u8; 32]> {
    let hex_clean = addr.strip_prefix("0x").unwrap_or(addr);
    let padded = format!("{:0>64}", hex_clean);
    let bytes = hex::decode(&padded).context("Invalid hex address")?;
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    let hex_clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(hex_clean).context("Invalid hex string")
}

// ============================================================================
// RLP ENCODING HELPERS (for legacy EVM transactions)
// ============================================================================

fn strip_leading_zeros(bytes: &[u8]) -> Vec<u8> {
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    bytes[start..].to_vec()
}

fn rlp_encode_u64(val: u64) -> Vec<u8> {
    if val == 0 {
        return vec![];
    }
    let bytes = val.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(8);
    bytes[start..].to_vec()
}

fn rlp_encode_item(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        vec![data[0]]
    } else if data.is_empty() {
        vec![0x80]
    } else if data.len() <= 55 {
        let mut out = vec![0x80 + data.len() as u8];
        out.extend_from_slice(data);
        out
    } else {
        let len_bytes = rlp_encode_u64(data.len() as u64);
        let mut out = vec![0xb7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(data);
        out
    }
}

fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    for item in items {
        payload.extend(rlp_encode_item(item));
    }

    if payload.len() <= 55 {
        let mut out = vec![0xc0 + payload.len() as u8];
        out.extend(payload);
        out
    } else {
        let len_bytes = rlp_encode_u64(payload.len() as u64);
        let mut out = vec![0xf7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend(payload);
        out
    }
}
