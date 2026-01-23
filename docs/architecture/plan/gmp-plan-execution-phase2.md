# Phase 2: SVM Prototype (3-4 days)

**Status:** Not Started
**Depends On:** Phase 1
**Blocks:** Phase 3

---

## Commits

### Commit 1: Implement MockLayerZeroEndpoint for Solana testing

**Files:**

- `intent-frameworks/svm/programs/mock-lz-endpoint/src/lib.rs`
- `intent-frameworks/svm/programs/mock-lz-endpoint/tests/mock_tests.rs`

**Tasks:**

- [ ] Implement `send` instruction that emits `MessageSent` event (no actual cross-chain)
- [ ] Implement `deliver_message` instruction for test/simulator to inject messages
- [ ] Implement trusted remote verification via PDA
- [ ] Track message nonces for realistic behavior
- [ ] Test `send` emits correct event with payload
- [ ] Test `deliver_message` calls receiver's `lz_receive`
- [ ] Test nonce tracking works correctly

**Test:**

```bash
# Run tests (auto-enters nix shell)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p mock-lz-endpoint --tests"
```

---

### Commit 2: Implement OutflowValidator program with tests

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs`
- `intent-frameworks/svm/programs/outflow-validator/tests/validator_tests.rs`

**Tasks:**

- [ ] Implement LayerZero OApp pattern in native Solana Rust
- [ ] Implement `lz_receive` to receive intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store intent requirements** in PDA (intent_id/step => {requirements, authorizedSolver})
- [ ] Implement `fulfill_intent` instruction for authorized solvers to call
- [ ] Instruction pulls tokens from authorized solver's wallet via SPL token transfer (solver includes transfer instruction in same transaction)
- [ ] Validate recipient, amount, token match stored requirements
- [ ] Validate solver matches authorized solver from stored requirements
- [ ] Forward tokens to user wallet
- [ ] Send GMP message to hub via `lz_send`
- [ ] Emit `FulfillmentSucceeded` or `FulfillmentFailed` events
- [ ] Test `lz_receive` stores requirements correctly
- [ ] Test `fulfill_intent` succeeds with matching params and authorized solver
- [ ] Test `fulfill_intent` fails with wrong recipient/amount/token
- [ ] Test `fulfill_intent` fails with unauthorized solver
- [ ] Test trusted remote verification rejects unknown sources
- [ ] Test atomic execution (transfer + validation + forwarding + GMP send in one transaction)
- [ ] Test idempotency: duplicate GMP message is ignored (requirements already stored)

**Test:**

```bash
# Run tests (auto-enters nix shell)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p outflow-validator --tests"
```

---

### Commit 3: Implement InflowEscrowGMP program with tests

**Files:**

- `intent-frameworks/svm/programs/escrow-gmp/src/lib.rs`
- `intent-frameworks/svm/programs/escrow-gmp/tests/escrow_tests.rs`

**Tasks:**

- [ ] Implement LayerZero OApp pattern in native Solana Rust
- [ ] Implement `lz_receive` for intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store requirements** (mapped by intent_id + step number)
- [ ] Implement `create_escrow_with_validation` - validates that requirements exist (from GMP message) and match escrow details, reverts if requirements don't exist or don't match
- [ ] Implement `lz_receive` for fulfillment proof from hub
- [ ] Implement automatic escrow release on fulfillment proof receipt
- [ ] Send `EscrowConfirmation` message back to hub on creation
- [ ] Test intent requirements storage via `lz_receive`
- [ ] Test escrow creation validates against requirements
- [ ] Test escrow creation fails with mismatched requirements
- [ ] Test escrow confirmation message sent on creation
- [ ] Test escrow release on fulfillment proof receipt
- [ ] Test idempotency: duplicate GMP message is ignored (requirements already stored)
- [ ] Test escrow creation reverts if requirements don't exist
- [ ] Test escrow creation reverts if requirements don't match

**Test:**

```bash
# Run tests (auto-enters nix shell)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p escrow-gmp --tests"
```

---

### Commit 4: Add integration test package and outflow E2E test

**Files:**

- `intent-frameworks/svm/tests/Cargo.toml` (new integration test package)
- `intent-frameworks/svm/tests/src/lib.rs`
- `intent-frameworks/svm/tests/tests/outflow_e2e.rs`
- `intent-frameworks/svm/Cargo.toml` (add `tests` to workspace members)

**Tasks:**

- [ ] Create `tests` package with dev-dependencies on `mock-lz-endpoint`, `outflow-validator`, `solana-program-test`
- [ ] Add to workspace members in root Cargo.toml
- [ ] Deploy mock endpoint + OutflowValidator in test setup
- [ ] Simulate hub sending intent requirements via mock `deliver_message`
- [ ] Solver calls `validate_and_send` with correct params
- [ ] Verify `ValidationSucceeded` event emitted
- [ ] Test validation fails with incorrect params
- [ ] Test full flow: requirements → validate → success

**Test:**

```bash
# Run outflow integration test
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p tests --test outflow_e2e"
```

---

### Commit 5: Add inflow end-to-end integration test

**Files:**

- `intent-frameworks/svm/tests/tests/inflow_e2e.rs`

**Tasks:**

- [ ] Deploy mock endpoint + InflowEscrowGMP in test setup
- [ ] Simulate hub sending intent requirements via mock `deliver_message`
- [ ] Requester creates escrow with matching params
- [ ] Verify escrow created and confirmation message sent
- [ ] Simulate hub sending fulfillment proof via mock `deliver_message`
- [ ] Verify escrow releases to solver automatically
- [ ] Test full flow: requirements → escrow → fulfill → release

**Test:**

```bash
# Run inflow integration test
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p tests --test inflow_e2e"
```

---

### Commit 6: Add deployment scripts for devnet

**Files:**

- `intent-frameworks/svm/scripts/deploy-outflow-validator.sh`
- `intent-frameworks/svm/scripts/deploy-escrow-gmp.sh`
- `intent-frameworks/svm/scripts/configure-trusted-remotes.sh`

**Tasks:**

- [ ] Script to deploy OutflowValidator to Solana devnet
- [ ] Script to deploy InflowEscrowGMP to Solana devnet
- [ ] Script to configure trusted remotes (hub address via PDA)
- [ ] Add deployment verification
- [ ] Add dry-run mode for testing without actual deployment

**Test:**

```bash
cd intent-frameworks/svm && ./scripts/deploy-outflow-validator.sh --dry-run
cd intent-frameworks/svm && ./scripts/deploy-escrow-gmp.sh --dry-run
```

---

### Commit 7: Deploy to Solana devnet and verify

**Files:**

- `docs/architecture/plan/gmp-devnet-deployment.md`

**Tasks:**

- [ ] Deploy OutflowValidator to Solana devnet
- [ ] Deploy InflowEscrowGMP to Solana devnet
- [ ] Configure trusted remotes (Movement testnet hub address)
- [ ] Verify programs on Solana Explorer
- [ ] Document deployed program IDs

**Test:**

```bash
# Verify deployment
nix develop ./nix -c bash -c "solana program show <OUTFLOW_VALIDATOR_PROGRAM_ID> --url devnet"
nix develop ./nix -c bash -c "solana program show <ESCROW_GMP_PROGRAM_ID> --url devnet"

# Smoke test: read program state
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo run --bin read-config -- --cluster devnet"
```

---

## Run All Tests

```bash
# Build all programs (auto-enters nix shell)
./intent-frameworks/svm/scripts/build.sh

# Unit tests (all GMP packages)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p gmp-common -p mock-lz-endpoint -p outflow-validator -p escrow-gmp --tests"

# Integration tests
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p tests --test outflow_e2e"
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p tests --test inflow_e2e"

# Verify existing intent_escrow still works
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p intent_escrow --tests"
```

---

## Reference Implementations

### MockLayerZeroEndpoint (Solana - Native)

```rust
// intent-frameworks/svm/programs/mock-lz-endpoint/src/lib.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum MockLzInstruction {
    Send {
        dst_chain_id: u16,
        destination: Vec<u8>,
        payload: Vec<u8>,
    },
    DeliverMessage {
        src_chain_id: u16,
        src_address: Vec<u8>,
        nonce: u64,
        payload: Vec<u8>,
    },
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MockLzInstruction::try_from_slice(instruction_data)?;

    match instruction {
        MockLzInstruction::Send { dst_chain_id, destination, payload } => {
            let account_iter = &mut accounts.iter();
            let sender = next_account_info(account_iter)?;

            // Emit event via logging (parsed by indexer)
            msg!("MessageSent: dst={}, sender={}", dst_chain_id, sender.key);
            msg!("payload: {:?}", payload);
            Ok(())
        }
        MockLzInstruction::DeliverMessage { src_chain_id, src_address, nonce, payload } => {
            let account_iter = &mut accounts.iter();
            let receiver_program = next_account_info(account_iter)?;

            // CPI to receiver's lz_receive instruction
            // Build instruction and invoke...
            msg!("DeliverMessage: src={}, nonce={}", src_chain_id, nonce);
            Ok(())
        }
    }
}
```

### Integration Test Example (solana-program-test)

```rust
// intent-frameworks/svm/tests/tests/outflow_e2e.rs
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

#[tokio::test]
async fn test_outflow_cross_chain() {
    // 1. Setup: Deploy mock endpoint + OutflowValidator
    let mut program_test = ProgramTest::new(
        "mock_lz_endpoint",
        mock_lz_endpoint::id(),
        processor!(mock_lz_endpoint::process_instruction),
    );
    program_test.add_program(
        "outflow_validator",
        outflow_validator::id(),
        processor!(outflow_validator::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // 2. Simulate hub sending intent requirements via deliver_message
    let deliver_ix = build_deliver_message_instruction(
        &mock_lz_endpoint::id(),
        src_chain_id,
        src_address,
        nonce,
        intent_requirements_payload,
    );
    let tx = Transaction::new_signed_with_payer(
        &[deliver_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(tx).await.unwrap();

    // 3. Solver validates on Solana
    let validate_ix = build_validate_and_send_instruction(
        &outflow_validator::id(),
        intent_id,
        recipient,
        amount,
    );
    let tx = Transaction::new_signed_with_payer(
        &[validate_ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let result = banks_client.process_transaction(tx).await;

    // 4. Verify validation succeeded
    assert!(result.is_ok());
}
```

---

## Documentation Update

At the end of Phase 2, update:

- [ ] `docs/architecture/plan/gmp-devnet-deployment.md` - Document deployed SVM program IDs
- [ ] `docs/svm/` - Add GMP program usage documentation
- [ ] `intent-frameworks/svm/README.md` - Update with new GMP programs
- [ ] Review conception documents for accuracy after changes
- [ ] Check if other files reference SVM escrow flow and update them

---

## Exit Criteria

- [ ] All 7 commits merged to feature branch
- [ ] All SVM unit + integration tests pass (`cargo test --workspace`)
- [ ] Programs deployed to Solana devnet
- [ ] Programs verified on Solana Explorer
- [ ] Smoke test on devnet passes
- [ ] Documentation updated
