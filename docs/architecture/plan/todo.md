# Future Work

## Testing

1. **Test Improvements**
   - Add timeout scenario tests
   - Test with multiple concurrent intents (unit tests in `coordinator/tests/monitor_tests.rs`, `integrated-gmp/tests/monitor_tests.rs`)
   - Add negative test cases (rejected intents, failed fulfillments)

## Naming Consistency

- Align entity names across VMs (MVM, EVM, SVM) and E2E test scripts
- Current inconsistencies: `approver_evm_pubkey_hash` vs `relay address`, `APPROVER_ADDR` vs `RELAY_ETH_ADDRESS`, Hardhat account indices vs Aptos profiles vs Solana key-pair files
- Define canonical role names (deployer, requester, solver, relay) and use them consistently in configs, scripts, variable names, and log messages

## Documentation

1. Finalize node bootstrapping instructions (ports, genesis, module publish) for both chains
2. Add more comprehensive API documentation
3. Add troubleshooting guide for common issues

## Move-intent-framework

- Add more intent types and use cases
- Optimize gas costs

## Chain-Clients Extraction

1. **Solver SVM sync→async migration** — **Deferred (intentional)**
   - Solver's `ConnectedSvmClient` keeps sync query methods (`is_escrow_released`, `get_token_balance`, `get_native_balance`) using `solana_client::RpcClient` directly instead of delegating to the shared async `SvmClient`
   - MVM/EVM solver clients delegate because they only need the async shared client (fulfillment uses external CLIs). SVM builds/signs transactions in-process via `solana_sdk`, which requires the blocking `RpcClient` — so query methods reuse it
   - Wrapping async in `block_on()` adds complexity with no functional benefit. Revisit if Solana SDK gains a stable async client
   - See: `solver/src/chains/connected_svm_client.rs` module doc comment

## Coordinator & Integrated-GMP

1. **Performance Testing**
   - Load testing coordinator and integrated-gmp APIs
   - Stress testing coordinator event monitoring
   - Memory usage monitoring (both services)

2. **Validation Hardening (Integrated-GMP)**
   - Add metadata and timeout checks
   - Support multiple concurrent intents robustly
   - Improve error handling and reporting

3. **Event Discovery Improvements (Coordinator)**
   - Currently polls known accounts via `/v1/accounts/{address}/transactions`
   - Incomplete coverage (misses unlisted accounts)
   - Manual configuration (requires prelisting emitters)
   - Not scalable (unsuitable for many users)
   - Consider using event streams or indexer integration

4. **Feature Enhancements**
   - Add "ok" endpoint for a given `intent_id` to signal escrow is satisfied so solver can commit on hub (integrated-gmp)
   - Add support for more chain types (coordinator + integrated-gmp)
   - Add metrics and observability (both services)
