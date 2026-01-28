//! Unit tests for SVM client functions
//!
//! These tests verify that the SVM client handles program account responses,
//! including base64 decoding and escrow parsing.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::BorshSerialize;
use serde_json::json;
use solana_program::pubkey::Pubkey;
use trusted_gmp::svm_client::{EscrowAccount, SvmClient};
use wiremock::matchers::{body_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn build_escrow_base64() -> String {
    let escrow = EscrowAccount {
        discriminator: [0u8; 8],
        requester: Pubkey::new_from_array([1u8; 32]),
        token_mint: Pubkey::new_from_array([2u8; 32]),
        amount: 123,
        is_claimed: false,
        expiry: 999,
        reserved_solver: Pubkey::new_from_array([3u8; 32]),
        intent_id: [4u8; 32],
        bump: 1,
    };
    STANDARD.encode(escrow.try_to_vec().expect("borsh serialize escrow"))
}

/// What is tested: get_all_escrows() parses program account responses into EscrowAccount
/// Why: Ensure base64 decoding + Borsh parsing stays wired correctly for SVM accounts
#[tokio::test]
async fn test_get_all_escrows_parses_program_accounts() {
    let mock_server = MockServer::start().await;
    let program_id = Pubkey::new_unique().to_string();
    let account_data = build_escrow_base64();

    let response = json!({
        "jsonrpc": "2.0",
        "result": [{
            "pubkey": Pubkey::new_unique().to_string(),
            "account": {
                "data": [account_data, "base64"]
            }
        }],
        "error": null
    });

    Mock::given(method("POST"))
        .and(body_json(json!({
            "jsonrpc": "2.0",
            "method": "getProgramAccounts",
            "params": [
                program_id,
                { "encoding": "base64" }
            ],
            "id": 1
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), &program_id)
        .expect("Failed to create SvmClient");
    let escrows = client.get_all_escrows().await.expect("Should fetch escrows");
    assert_eq!(escrows.len(), 1);
    assert_eq!(escrows[0].escrow.amount, 123);
    assert_eq!(escrows[0].escrow.intent_id, [4u8; 32]);
}
