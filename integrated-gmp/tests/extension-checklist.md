# Integrated GMP Test Completeness

> Conventions, legend, and full index: [Checklist Guide](../../docs/checklist-guide.md)

## *vm_tests.rs (module entrypoints)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | N/A | [x] | N/A |

## tests/*vm/config_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_evm_chain_config_structure | N/A | [x] | N/A |
| 2 | test_connected_chain_evm_with_values | N/A | [x] | N/A |
| 3 | test_evm_config_serialization | N/A | [x] | N/A |
| 4 | test_evm_chain_config_with_all_fields | N/A | [x] | N/A |
| 5 | test_evm_config_loading | N/A | [x] | N/A |

## tests/*vm_client_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| | **ADDRESS NORMALIZATION TESTS** | | | |
| 1 | test_normalize_address_adds_prefix | [x] | N/A | N/A |
| 2 | test_normalize_address_preserves_existing_prefix | [x] | N/A | N/A |
| | **TRANSACTION HASH EXTRACTION TESTS** | | | |
| 3 | test_extract_transaction_hash_from_json_output | [x] | N/A | N/A |
| 4 | test_extract_transaction_hash_returns_none_when_missing | [x] | N/A | N/A |
| | **VM STATUS CHECKING TESTS** | | | |
| 5 | test_check_vm_status_success_result_wrapper | [x] | N/A | N/A |
| 6 | test_check_vm_status_failure_result_wrapper | [x] | N/A | N/A |
| 7 | test_check_vm_status_success_top_level | [x] | N/A | N/A |
| | **PARSE VIEW BYTES TESTS** | | | |
| 8 | test_parse_view_bytes_hex_string | [x] | N/A | N/A |
| 9 | test_parse_view_bytes_hex_string_no_prefix | [x] | N/A | N/A |
| 10 | test_parse_view_bytes_json_array | [x] | N/A | N/A |
| 11 | test_parse_view_bytes_empty_array | [x] | N/A | N/A |

## tests/*vm_relay_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| | **SVM PUBKEY PARSING TESTS** | | | |
| 1 | test_parse_svm_pubkey_from_hex_format | N/A | N/A | [x] |
| 2 | test_parse_svm_pubkey_from_base58_format | N/A | N/A | [x] |
| | **SVM MESSAGE PARSING TESTS** | | | |
| 3 | test_parse_svm_message_sent_log_format | N/A | N/A | [x] |
| 4 | test_non_message_sent_log_is_ignored | N/A | N/A | [x] |
| 5 | test_solana_pubkey_to_hex_conversion | N/A | N/A | [x] |
| | **RELAY CONFIG TESTS** | | | |
| 6 | test_relay_config_extracts_mvm_connected_chain | [x] | N/A | N/A |
| 7 | test_relay_config_extracts_both_connected_chains | [x] | [ ] | N/A |
| 8 | test_relay_config_handles_missing_mvm_connected | N/A | N/A | [x] |
| 9 | test_relay_config_extracts_evm_connected_chain | N/A | [x] | N/A |
| | **FULFILLMENT PROOF PAYLOAD PARSING TESTS** | | | |
| 10 | test_fulfillment_proof_payload_intent_id_extraction | N/A | N/A | [x] |
| 11 | test_fulfillment_proof_payload_minimum_length | N/A | N/A | [x] |
| | **ATA DERIVATION TESTS** | | | |
| 12 | test_ata_derivation_formula | N/A | N/A | [x] |
| 13 | test_ata_derivation_is_deterministic | N/A | N/A | [x] |
| 14 | test_ata_differs_by_owner | N/A | N/A | [x] |
| | **EVM EVENT TOPIC TESTS** | | | |
| 15 | test_evm_event_topic_produces_known_keccak_hash | N/A | [ ] | N/A |
| 16 | test_evm_event_topic_is_deterministic | N/A | [ ] | N/A |
| | **EVM ABI ENCODING TESTS** | | | |
| 17 | test_evm_encode_deliver_message_calldata | N/A | [ ] | N/A |
| 18 | test_evm_encode_deliver_message_with_empty_payload | N/A | [ ] | N/A |
| | **EVM LOG PARSING TESTS** | | | |
| 19 | test_parse_evm_message_sent_log | N/A | [ ] | N/A |
| 20 | test_evm_message_sent_log_short_data_ignored | N/A | [ ] | N/A |
| 21 | test_evm_message_sent_log_missing_topics_ignored | N/A | [ ] | N/A |
| | **RLP ENCODING TESTS** | | | |
| 22 | test_rlp_encode_u64_known_values | N/A | [ ] | N/A |
| 23 | test_rlp_encode_item_short_string | N/A | [ ] | N/A |
| 24 | test_rlp_encode_list_basic | N/A | [ ] | N/A |
| | **MVM OUTBOX MESSAGE PARSING TESTS** | | | |
| 25 | test_mvm_get_message_response_parsing | [ ] | N/A | N/A |
| 26 | test_mvm_get_next_nonce_response_parsing | [ ] | N/A | N/A |
| | **SVM ACCOUNT DATA PARSING TESTS** | | | |
| 27 | test_svm_outbound_nonce_account_layout | N/A | N/A | [ ] |
| 28 | test_svm_outbound_nonce_account_too_short | N/A | N/A | [ ] |
| 29 | test_svm_message_account_field_extraction | N/A | N/A | [ ] |
| 30 | test_svm_message_account_discriminator_check | N/A | N/A | [ ] |
| 31 | test_svm_message_account_payload_truncation | N/A | N/A | [ ] |
