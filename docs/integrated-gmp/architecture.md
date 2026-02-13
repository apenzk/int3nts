# GMP Architecture

## Overview

The GMP (Generic Message Passing) system enables cross-chain communication between the hub chain (MVM) and connected chains (MVM, EVM, SVM). Validation logic runs **on-chain** in smart contracts; the Integrated GMP service is a **relay** that moves messages between chains.

```text
Hub Chain (MVM)                 Integrated GMP Relay              Connected Chain
────────────────                ────────────────────              ───────────────
Contract calls gmpSend()    -->  Watches MessageSent events   -->  Calls deliver_message()
                           <--  Watches MessageSent events   <--  Contract calls gmpSend()
```

## Message Flow

### Inflow (Connected Chain --> Hub)

Tokens locked on connected chain, desired on hub.

```text
1. Hub: create_inflow_intent()
   └─> Sends IntentRequirements (0x01) via GMP to connected chain

2. Connected Chain: receive IntentRequirements
   └─> Stores requirements on-chain

3. Connected Chain: create_escrow_with_validation()
   └─> Validates against stored requirements
   └─> Sends EscrowConfirmation (0x02) via GMP to hub

4. Hub: receive EscrowConfirmation
   └─> Sets escrow_confirmed = true (gates fulfillment)

5. Hub: fulfill_inflow_intent()
   └─> Sends FulfillmentProof (0x03) via GMP to connected chain

6. Connected Chain: receive FulfillmentProof
   └─> Auto-releases escrowed funds to solver
```

### Outflow (Hub --> Connected Chain)

Tokens locked on hub, desired on connected chain.

```text
1. Hub: create_outflow_intent()
   └─> Locks tokens on hub
   └─> Sends IntentRequirements (0x01) via GMP to connected chain

2. Connected Chain: receive IntentRequirements
   └─> Stores requirements on-chain

3. Connected Chain: solver calls fulfill_intent()
   └─> Validates against stored requirements
   └─> Transfers tokens to requester
   └─> Sends FulfillmentProof (0x03) via GMP to hub

4. Hub: receive FulfillmentProof
   └─> Sets fulfillment_proof_received = true

5. Hub: solver calls fulfill_outflow_intent()
   └─> Claims locked tokens on hub
```

## Relay Architecture

### Polling

The relay polls each configured chain for outbound messages:

| Chain | Polling Method | State Tracked |
| ----- | -------------- | ------------- |
| MVM (hub) | `get_next_nonce()` + `get_message(nonce)` view functions | Last processed nonce |
| MVM (connected) | Same as hub | Last processed nonce |
| EVM | `eth_getLogs` for `MessageSent` events (10-block ranges) | Last polled block |
| SVM | `OutboundNonceAccount` PDAs per destination chain | Next nonce per destination |

Default polling interval: 2000ms (configurable via `polling_interval_ms`).

### Message Delivery

| Destination | Delivery Method |
| ----------- | --------------- |
| MVM | CLI-based `aptos move run` calling `deliver_message_entry` |
| EVM | ABI-encoded `deliverMessage()` via `eth_sendRawTransaction` |
| SVM | `DeliverMessage` Solana instruction submission |

### Authorization

The relay must be authorized on each chain's GMP endpoint before it can deliver messages:

1. Relay starts and checks authorization on all configured chains
2. Calls `is_relay_authorized(relay_addr)` on each chain
3. Fails fast if any chain reports unauthorized
4. To authorize: call `add_relay(relay_addr)` on each chain's GMP contract

### Error Handling

The relay distinguishes between permanent and transient errors:

**Permanent** (skips message, advances cursor):

- `E_ALREADY_DELIVERED` -- message already processed (idempotent)
- `E_UNKNOWN_REMOTE_GMP_ENDPOINT` -- destination chain not configured on receiving contract

**Transient** (backs off, retries on next poll):

- Network failures, RPC timeouts
- Transaction submission failures
- VM execution failures

## Configuration

### Config File Structure

```toml
[hub_chain]
name = "Hub Chain"
rpc_url = "http://127.0.0.1:8080"
chain_id = 1
intent_module_addr = "0x..."

[connected_chain_mvm]
rpc_url = "http://127.0.0.1:8080"
chain_id = 2
intent_module_addr = "0x..."

[connected_chain_evm]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337
escrow_contract_addr = "0x..."
gmp_endpoint_addr = "0x..."
approver_evm_pubkey_hash = "0x..."

[connected_chain_svm]
rpc_url = "http://127.0.0.1:8899"
chain_id = 1001
escrow_program_id = "..."
outflow_program_id = "..."
gmp_endpoint_program_id = "..."

[integrated_gmp]
private_key_env = "INTEGRATED_GMP_PRIVATE_KEY"
public_key_env = "INTEGRATED_GMP_PUBLIC_KEY"
polling_interval_ms = 2000

[api]
host = "127.0.0.1"
port = 3334
```

### Configuration Loading Priority

1. Environment variable: `INTEGRATED_GMP_CONFIG_PATH`
2. CLI flag: `--config <path>`
3. CLI flag: `--testnet` (uses `config/integrated-gmp_testnet.toml`)
4. Default: `config/integrated-gmp.toml`

### Key Management

The relay uses a single Ed25519 keypair from which it derives addresses for all chain types:

- **MVM**: Ed25519 public key as Move address
- **EVM**: Secp256k1 key derived from Ed25519, Ethereum address via keccak256
- **SVM**: Ed25519 public key as Solana address

Keys are loaded from environment variables specified in config (`private_key_env`, `public_key_env`), stored as Base64-encoded Ed25519 bytes.

## Testnet Deployment

Current testnet addresses (from `config/integrated-gmp_testnet.toml`):

| Chain | Component | Address |
| ----- | --------- | ------- |
| MVM (Hub) | Intent module | `0x84061212c25b0371e8f358bcf9bf6cd919c7dc1e2ac553c7229f059d9b520caf` |
| MVM (Hub) | RPC | `https://testnet.movementnetwork.xyz/v1` |
| EVM (Base Sepolia) | IntentGmp | `0x673e3F207d41d09e019D9e68116561A3392a6512` |
| EVM (Base Sepolia) | IntentInflowEscrow | `0x7011b77326635f41Fe1Ed8C6fb85c3198287B18A` |
| SVM (Devnet) | GMP program | `fXyBzUDtsDX8FQXcefoX6ahBqs8nptjxoqVM8n4PeJq` |
| SVM (Devnet) | Escrow program | `98W9iZ6rUSFBHWri7q5EmvNqdUCbq2KENbvGqrV5ZYH1` |
| SVM (Devnet) | Outflow program | `DTzkzw4fU2e1F1hsiK6Qu2ZBJ8ntjPv4QvkoinT77Jbx` |

## Security Model

| Aspect | Detail |
| ------ | ------ |
| **Trust level** | Relay can forge messages to integrated GMP endpoints |
| **Mitigation** | All validation logic is on-chain and transparent |
| **Key exposure** | Operator wallets for gas only, not approval authority |
| **Future improvement** | Replace with external GMP provider endpoints (config change only) |

The relay can deliver arbitrary messages to integrated GMP endpoints. All validation happens on-chain, making it auditable and transparent. A compromised relay can still cause harm by delivering forged messages, but this is mitigated by on-chain validation of message contents.
