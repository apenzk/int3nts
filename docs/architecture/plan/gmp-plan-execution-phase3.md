# Phase 3: Multi-Chain Expansion (5-7 days)

**Status:** Not Started
**Depends On:** Phase 2
**Blocks:** Phase 4

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Add LayerZero OApp base for Movement (MVM)

**Files:**

- `intent-frameworks/mvm/sources/layerzero/oapp.move`
- `intent-frameworks/mvm/sources/layerzero/endpoint.move`
- `intent-frameworks/mvm/sources/mocks/mock_lz_endpoint.move`
- `intent-frameworks/mvm/sources/tests/layerzero_tests.move`

**Tasks:**

- [ ] Port LayerZero OApp pattern to Move
- [ ] Implement `lz_receive()` entry function
- [ ] Implement `lz_send()` internal function
- [ ] Implement trusted remote verification
- [ ] Implement mock endpoint for testing
- [ ] Test send/receive with mock endpoint

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.**

---

### Commit 2: Integrate GMP into MVM hub intent contract

**Files:**

- `intent-frameworks/mvm/sources/intent.move`
- `intent-frameworks/mvm/sources/tests/intent_gmp_tests.move`

**Tasks:**

- [ ] Add `send_intent_requirements()` - calls `lz_send()` on intent creation
- [ ] Add `send_fulfillment_proof()` - calls `lz_send()` on fulfillment
- [ ] Add `receive_escrow_confirmation()` - called by `lz_receive()`
- [ ] Gate fulfillment on escrow confirmation receipt
- [ ] Test message encoding matches SVM/EVM schema
- [ ] Test fulfillment blocked without escrow confirmation
- [ ] Test state updates on GMP message receipt

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.**

---

### Commit 3: Implement EVM contracts with GMP (OutflowValidator)

**Files:**

- `intent-frameworks/evm/contracts/mocks/MockLayerZeroEndpoint.sol`
- `intent-frameworks/evm/contracts/OutflowValidator.sol`
- `intent-frameworks/evm/test/OutflowValidator.test.ts`

**Tasks:**

- [ ] Implement MockLayerZeroEndpoint for EVM testing
- [ ] Inherit from LayerZero `OApp` base contract
- [ ] Implement `lzReceive()` to receive intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store intent requirements** in mapping (intent_id/step => {requirements, authorizedSolver})
- [ ] Implement `fulfillIntent(intent_id, token, amount)` for authorized solvers to call
- [ ] Pull tokens from authorized solver's wallet via `transferFrom(authorizedSolver, contract, amount)` (requires prior approval)
- [ ] Validate recipient, amount, token match stored requirements
- [ ] Validate solver matches authorized solver from stored requirements
- [ ] Forward tokens to user wallet
- [ ] Send GMP message to hub via `lzSend()`
- [ ] Test all validation scenarios
- [ ] Test `transferFrom()` fails without approval
- [ ] Test fulfillment fails with unauthorized solver
- [ ] Test atomic execution (transferFrom + validation + forwarding + GMP send in one transaction)

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.**

---

### Commit 4: Implement EVM contracts with GMP (InflowEscrowGMP)

**Files:**

- `intent-frameworks/evm/contracts/InflowEscrowGMP.sol`
- `intent-frameworks/evm/test/InflowEscrowGMP.test.ts`

**Tasks:**

- [ ] Inherit from LayerZero `OApp` base contract
- [ ] Implement `lzReceive()` for intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store requirements** (mapped by intent_id + step number)
- [ ] Implement `createEscrowWithValidation()` - validates that requirements exist and match escrow details
- [ ] Implement automatic escrow release on fulfillment proof receipt
- [ ] Send `EscrowConfirmation` message back to hub on creation
- [ ] Test all escrow scenarios
- [ ] Test idempotency: duplicate GMP message is ignored (requirements already stored)
- [ ] Test escrow creation reverts if requirements don't exist
- [ ] Test escrow creation reverts if requirements don't match

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.**

---

### Commit 5: Implement LayerZero simulator in trusted-gmp

**Files:**

- `trusted-gmp/src/layerzero_simulator.rs`
- `trusted-gmp/src/main.rs`
- `trusted-gmp/src/tests/simulator_tests.rs`

**Tasks:**

- [ ] Add `LayerZeroSimulator` struct
- [ ] Watch for `MessageSent` events on all chains (MVM, SVM, EVM)
- [ ] Deliver messages by calling `lzReceive` / `deliver_message`
- [ ] Support configurable chain RPCs and mock endpoints
- [ ] Integrate into trusted-gmp binary as `--mode simulator`
- [ ] Test event parsing and message delivery

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.**

---

### Commit 6: Add cross-chain E2E test: MVM ↔ SVM outflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-svm-outflow.sh`
- `testing-infra/ci-e2e/e2e-tests-gmp/test-helpers.sh`

**Tasks:**

- [ ] Set up test environment with mock endpoints on both chains
- [ ] Start LayerZero simulator in background
- [ ] Create intent on MVM hub
- [ ] Verify requirements message sent to SVM
- [ ] Solver validates on SVM
- [ ] Verify success message sent back to MVM

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.**

---

### Commit 7: Add cross-chain E2E test: MVM ↔ SVM inflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-svm-inflow.sh`

**Tasks:**

- [ ] Create intent on MVM hub (inflow type)
- [ ] Verify requirements message sent to SVM
- [ ] Requester creates escrow on SVM
- [ ] Verify escrow confirmation sent back to MVM
- [ ] Solver fulfills on MVM hub
- [ ] Verify fulfillment proof sent to SVM
- [ ] Verify escrow releases on SVM

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 8.**

---

### Commit 8: Add cross-chain E2E test: MVM ↔ EVM

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-outflow.sh`
- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-inflow.sh`

**Tasks:**

- [ ] Outflow: MVM intent → EVM validation → success
- [ ] Inflow: MVM intent → EVM escrow → MVM fulfillment → EVM release
- [ ] Use simulator for cross-chain message relay

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before Phase 3 is complete.**

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests (MVM, EVM, SVM - inflow + outflow, plus new GMP cross-chain tests) must pass before merging.**

---

## Reference Implementations

### MVM Intent GMP Contract

```move
// intent-frameworks/mvm/sources/intent_gmp.move
module intent::intent_gmp {
    use layerzero::endpoint;

    // Called when intent is fulfilled on hub
    public fun send_fulfillment_proof(
        intent_id: u64,
        solver: address,
        amount: u64
    ) {
        let payload = encode_fulfillment(intent_id, solver, amount);
        // Calls real LZ endpoint in production, mock in CI
        endpoint::send(
            connected_chain_id,
            escrow_contract_address,
            payload
        );
    }

    // LayerZero calls this on message receipt
    public fun lz_receive(
        src_chain_id: u64,
        src_address: vector<u8>,
        payload: vector<u8>
    ) {
        let (escrow_id, amount, solver) = decode_payload(payload);
        validate_and_release_escrow(escrow_id, amount, solver);
    }
}
```

### EVM MockLayerZeroEndpoint

```solidity
// intent-frameworks/evm/contracts/mocks/MockLayerZeroEndpoint.sol
contract MockLayerZeroEndpoint {
    event MessageSent(
        uint16 dstChainId,
        bytes destination,
        bytes payload,
        address sender
    );

    function send(
        uint16 _dstChainId,
        bytes calldata _destination,
        bytes calldata _payload,
        address payable,
        address,
        bytes calldata
    ) external payable {
        emit MessageSent(_dstChainId, _destination, _payload, msg.sender);
    }

    function deliverMessage(
        address receiver,
        uint16 srcChainId,
        bytes calldata srcAddress,
        uint64 nonce,
        bytes calldata payload
    ) external {
        ILayerZeroReceiver(receiver).lzReceive(srcChainId, srcAddress, nonce, payload);
    }
}
```

### LayerZero Simulator

```rust
// trusted-gmp/src/layerzero_simulator.rs
pub struct LayerZeroSimulator {
    hub_rpc: String,
    connected_rpcs: HashMap<u64, String>,
    mock_endpoints: HashMap<u64, Address>,
}

impl LayerZeroSimulator {
    pub async fn run(&self) -> Result<()> {
        info!("Starting LayerZero simulator for CI testing");
        loop {
            for (chain_id, rpc_url) in &self.connected_rpcs {
                let events = self.query_message_sent_events(*chain_id, rpc_url).await?;
                for event in events {
                    self.deliver_message(event).await?;
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
```

### Simulator Configuration

```toml
# trusted-gmp/config.ci.toml
[mode]
type = "ci_simulator"

[ci_simulator]
enabled = true

[[ci_simulator.mock_endpoints]]
chain_id = 1
chain_type = "mvm"
rpc_url = "http://localhost:8545"
endpoint_address = "0x1234..."

[[ci_simulator.mock_endpoints]]
chain_id = 900  # Solana localnet
chain_type = "svm"
rpc_url = "http://localhost:8899"
endpoint_address = "<PROGRAM_ID>"

[[ci_simulator.mock_endpoints]]
chain_id = 84532  # Base Sepolia
chain_type = "evm"
rpc_url = "http://localhost:8546"
endpoint_address = "0x5678..."
```

### CI Pipeline

```yaml
# .github/workflows/gmp-tests.yml
jobs:
  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - name: Start local blockchain nodes
        run: |
          anvil --port 8545 --chain-id 1 &        # MVM sim
          solana-test-validator --reset &          # SVM
          anvil --port 8546 --chain-id 84532 &    # Base
          sleep 5

      - name: Deploy mock LayerZero endpoints
        run: |
          # Deploy to all three chains
          ./scripts/deploy-mocks.sh

      - name: Start simulator
        run: |
          cd trusted-gmp && cargo build --release
          ./target/release/trusted-gmp --config config.ci.toml &
          sleep 3

      - name: Run integration tests
        run: npm run test:integration
```

---

## Documentation Update

At the end of Phase 3, update:

- [ ] `docs/mvm/` - Add GMP integration documentation for MVM contracts
- [ ] `docs/evm/` - Add GMP integration documentation for EVM contracts
- [ ] `docs/testing/` - Document how to run LayerZero simulator for local testing
- [ ] `intent-frameworks/mvm/README.md` - Update with GMP module info
- [ ] `intent-frameworks/evm/README.md` - Update with GMP contract info
- [ ] Review conception documents for accuracy after changes
- [ ] Check if other files reference MVM/EVM flows and update them

---

## Exit Criteria

- [ ] All 8 commits merged to feature branch
- [ ] MVM GMP unit tests pass
- [ ] EVM GMP unit tests pass
- [ ] Trusted GMP simulator tests pass
- [ ] All cross-chain E2E tests pass (MVM↔SVM, MVM↔EVM)
- [ ] All three chains can send/receive GMP messages in test environment
- [ ] Documentation updated
