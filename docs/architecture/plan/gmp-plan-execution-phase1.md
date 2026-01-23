# Phase 1: Research & Design (2-3 days)

**Status:** Not Started
**Depends On:** Phase 0 (Verifier Separation)
**Blocks:** Phase 2

**Note:** Phase 0 separates the verifier into Coordinator and Trusted GMP. Phase 1 can now focus on on-chain validation design, knowing the architecture is cleaner.

---

## Commits

### Commit 1: Add gmp-common crate with LayerZero endpoint configuration

**Files:**

- `intent-frameworks/svm/programs/gmp-common/Cargo.toml`
- `intent-frameworks/svm/programs/gmp-common/src/lib.rs`
- `intent-frameworks/svm/programs/gmp-common/src/endpoints.rs`
- `docs/architecture/plan/gmp-endpoints.md`

**Tasks:**

- [ ] Create `gmp-common` library crate under `programs/` (auto-included via workspace glob `programs/*`)
- [ ] Define LayerZero endpoint address constants for Solana devnet
- [ ] Define LayerZero endpoint address constants for Movement testnet
- [ ] **Environment Configuration**: Support testnet vs mainnet endpoint addresses (configuration management)
- [ ] **Endpoint Verification**: Document how to verify endpoint addresses are legitimate LayerZero endpoints
- [ ] **Upgrade Handling**: Document procedures for handling endpoint contract upgrades
- [ ] Document chain IDs and endpoint addresses

**Test:**

```bash
# Build gmp-common (auto-included in workspace)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo build -p gmp-common"
```

---

### Commit 2: Define GMP message payload schemas (Rust)

**Files:**

- `intent-frameworks/svm/programs/gmp-common/src/messages.rs`
- `intent-frameworks/svm/programs/gmp-common/tests/message_tests.rs`
- `docs/architecture/plan/gmp-message-schemas.md`

**Tasks:**

- [ ] Define `IntentRequirements` struct (intent_id, recipient, amount, token, solver)
- [ ] Define `EscrowConfirmation` struct (intent_id, escrow_id, amount, token)
- [ ] Define `FulfillmentProof` struct (intent_id, solver, timestamp)
- [ ] Document message encoding format (Borsh serialization)
- [ ] Test encoding/decoding roundtrips correctly

**Test:**

```bash
# Build and test gmp-common
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo build -p gmp-common && cargo test -p gmp-common --tests"
```

---

### Commit 3: Add outflow validation program interface

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/Cargo.toml`
- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs` (interface only - stub implementations)

**Tasks:**

- [ ] Create Cargo.toml with dependencies on `gmp-common`, `solana-program`, `borsh`
- [ ] Define `fulfill_intent` instruction signature (for authorized solvers to call)
- [ ] Define `lz_receive` instruction for intent requirements
- [ ] Define events: `ValidationSucceeded`, `ValidationFailed`
- [ ] Define trusted remote verification via PDA
- [ ] Add stub implementations that return `Ok(())` (to pass build)

**Test:**

```bash
# Build outflow-validator (verifies it compiles)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo build -p outflow-validator"
```

---

### Commit 4: Add inflow escrow GMP program interface

**Files:**

- `intent-frameworks/svm/programs/escrow-gmp/Cargo.toml`
- `intent-frameworks/svm/programs/escrow-gmp/src/lib.rs` (interface only - stub implementations)

**Tasks:**

- [ ] Create Cargo.toml with dependencies on `gmp-common`, `solana-program`, `borsh`
- [ ] Define `receive_intent_requirements` instruction (GMP inbound)
- [ ] Define `create_escrow_with_validation` instruction
- [ ] Define `receive_fulfillment_proof` instruction (GMP inbound)
- [ ] Define `send_escrow_confirmation` instruction (GMP outbound)
- [ ] Add stub implementations that return `Ok(())` (to pass build)

**Test:**

```bash
# Build escrow-gmp (verifies it compiles)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo build -p escrow-gmp"
```

---

### Commit 5: Add hub intent contract GMP interface (MVM)

**Files:**

- `intent-frameworks/mvm/sources/interfaces/intent_gmp.move`

**Tasks:**

- [ ] Define `send_intent_requirements()` function (GMP outbound) - stub
- [ ] Define `receive_escrow_confirmation()` function (GMP inbound) - stub
- [ ] Define `send_fulfillment_proof()` function (GMP outbound) - stub

**Test:**

```bash
# Compile MVM contracts (verifies it compiles)
nix develop ./nix -c bash -c "cd intent-frameworks/mvm && movement move compile --named-addresses mvmt_intent=0x123"
```

---

### Commit 6: Add mock LayerZero endpoint interface (Solana)

**Files:**

- `intent-frameworks/svm/programs/mock-lz-endpoint/Cargo.toml`
- `intent-frameworks/svm/programs/mock-lz-endpoint/src/lib.rs` (interface only - stub implementations)

**Tasks:**

- [ ] Create Cargo.toml with dependencies on `gmp-common`, `solana-program`, `borsh`
- [ ] Define `send` instruction signature (emits event)
- [ ] Define `deliver_message` instruction for simulator to call
- [ ] Define `set_trusted_remote` for configuration
- [ ] Define `MessageSent` event
- [ ] Add stub implementations that return `Ok(())` (to pass build)

**Test:**

```bash
# Build mock-lz-endpoint (verifies it compiles)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo build -p mock-lz-endpoint"
```

---

### Commit 7: Add fee estimation script

**Files:**

- `intent-frameworks/svm/scripts/estimate-fees.ts`
- `docs/architecture/plan/gmp-fee-analysis.md`

**Tasks:**

- [ ] Script to estimate validation program compute units
- [ ] Script to estimate LayerZero message fees
- [ ] Document cost comparison vs current verifier system

**Test:**

```bash
# Run fee estimation script
nix develop ./nix -c bash -c "cd intent-frameworks/svm && npx ts-node scripts/estimate-fees.ts"
```

---

### Commit 8: Research LayerZero Solana integration

**Files:**

- `docs/architecture/plan/layerzero-solana-integration.md`

**Tasks:**

- [ ] Research LayerZero's Solana integration documentation
- [ ] Understand LayerZero's Solana endpoint interface (how it differs from EVM)
- [ ] Document how to implement OApp pattern in native Solana Rust (not just copy EVM pattern)
- [ ] Document LayerZero's Solana-specific message format (how LayerZero wraps Borsh messages)
- [ ] Document how to handle LayerZero's nonce tracking on Solana
- [ ] Document account model differences (Solana uses accounts, not contracts)
- [ ] Document program structure requirements (must implement OApp pattern manually)

**Test:**

```bash
# Documentation review - no automated test
# Manual: Review document for completeness
```

---

### Commit 9: Document LayerZero selection and integration details

**Files:**

- `docs/architecture/plan/gmp-selection.md`

**Tasks:**

- [ ] Document LayerZero v2 as final selection (decision already made)
- [ ] Document LayerZero Solana integration details and requirements
- [ ] Document Movement testnet/mainnet LayerZero endpoint addresses
- [ ] Document LayerZero fee structure for testnet chains
- [ ] Document OApp pattern implementation requirements for each chain type
- [ ] Document endpoint configuration for testnet vs mainnet

**Test:**

```bash
# Documentation review - no automated test
# Manual: Review document for completeness
```

---

## Run All Tests

```bash
# Build all GMP packages (library crates only - stubs compile)
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo build -p gmp-common -p mock-lz-endpoint -p outflow-validator -p escrow-gmp"

# Compile MVM contracts
nix develop ./nix -c bash -c "cd intent-frameworks/mvm && movement move compile --named-addresses mvmt_intent=0x123"

# Run message schema tests
nix develop ./nix -c bash -c "cd intent-frameworks/svm && cargo test -p gmp-common --tests"

# Verify existing programs still build
./intent-frameworks/svm/scripts/build.sh
```

---

## Documentation Update

At the end of Phase 1, update:

- [ ] `docs/architecture/plan/gmp-endpoints.md` - Document all LayerZero endpoint addresses
- [ ] `docs/architecture/plan/gmp-message-schemas.md` - Document message payload formats
- [ ] `docs/architecture/plan/gmp-selection.md` - Final LayerZero selection rationale
- [ ] Review conception documents for accuracy after changes
- [ ] Check if other files reference old message formats and update them

---

## Exit Criteria

- [ ] All 9 commits merged to feature branch
- [ ] All SVM program interfaces build without errors
- [ ] All MVM interfaces compile without errors
- [ ] Message schema encoding tests pass
- [ ] Fee analysis document complete
- [ ] GMP selection document reviewed and approved

---

## Reference: GMP Protocol Interfaces

### LayerZero

```solidity
interface ILayerZeroEndpoint {
    function send(
        uint16 _dstChainId,
        bytes calldata _destination,
        bytes calldata _payload,
        address payable _refundAddress,
        address _zroPaymentAddress,
        bytes calldata _adapterParams
    ) external payable;
}

interface ILayerZeroReceiver {
    function lzReceive(
        uint16 _srcChainId,
        bytes calldata _srcAddress,
        uint64 _nonce,
        bytes calldata _payload
    ) external;
}
```

### Axelar

```solidity
interface IAxelarGateway {
    function callContract(
        string calldata destinationChain,
        string calldata contractAddress,
        bytes calldata payload
    ) external;
}

interface IAxelarExecutable {
    function execute(
        bytes32 commandId,
        string calldata sourceChain,
        string calldata sourceAddress,
        bytes calldata payload
    ) external;
}
```

### Wormhole

```solidity
interface IWormhole {
    function publishMessage(
        uint32 nonce,
        bytes memory payload,
        uint8 consistencyLevel
    ) external payable returns (uint64 sequence);
}
```

### CCIP

```solidity
interface IRouterClient {
    function ccipSend(
        uint64 destinationChainSelector,
        Client.EVM2AnyMessage calldata message
    ) external payable returns (bytes32);
}

interface CCIPReceiver {
    function ccipReceive(
        Client.Any2EVMMessage calldata message
    ) external;
}
```
