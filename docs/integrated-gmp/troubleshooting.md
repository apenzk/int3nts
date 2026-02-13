# Troubleshooting

Common issues and solutions for the GMP relay and cross-chain message delivery.

## Relay Won't Start

### Relay not authorized

```text
ERROR: Relay {addr} is NOT authorized on {chain}
```

**Cause**: The relay's address hasn't been added to the GMP endpoint's authorized relay list.

**Fix**: Call `add_relay(relay_addr)` on each chain's GMP contract:

- **MVM**: `aptos move run --function-id {module}::gmp_endpoint::add_relay --args address:{relay_addr}`
- **EVM**: Call `gmpEndpoint.addRelay(relayAddr)` from the contract owner
- **SVM**: Call `add_relay` instruction with relay pubkey

### Missing config file

```text
ERROR: Config file not found
```

**Fix**: Set `INTEGRATED_GMP_CONFIG_PATH` environment variable or use `--config` flag. See [architecture.md](architecture.md#configuration) for config format.

### Missing private key

```text
ERROR: Environment variable {key_env} not set
```

**Fix**: Set the environment variables specified in config (`private_key_env`, `public_key_env`) with Base64-encoded Ed25519 key bytes.

## Messages Not Delivering

### Symptoms

- Solver stuck waiting for `has_outflow_requirements` or `is_escrow_confirmed`
- Intent expires without GMP messages arriving

### Diagnosis

Check relay logs for delivery attempts:

```bash
# Look for delivery errors
grep -i "deliver_message failed\|Permanent delivery failure\|ERROR" relay.log

# Check if relay is polling
grep -i "polling\|MessageSent\|new message" relay.log
```

### Common Causes

**1. Remote GMP endpoint not configured**

```text
WARN: Permanent delivery failure: E_UNKNOWN_REMOTE_GMP_ENDPOINT
```

**Fix**: Set the remote GMP endpoint on the destination chain's contract:

- **MVM**: `set_remote_gmp_endpoint(chain_id, remote_addr)`
- **EVM**: `setRemoteGmpEndpointAddr(chainId, remoteAddr)`
- **SVM**: `set_remote_endpoint` instruction

**2. Chain ID mismatch**

The `chain_id` in relay config must match the chain IDs used in intent creation. If they don't match, messages route to nowhere.

**Fix**: Verify chain IDs are consistent across:

- Relay TOML config
- Intent creation parameters (`offered_chain_id`, `desired_chain_id`)
- On-chain GMP endpoint configuration

**3. RPC endpoint down or rate-limited**

```text
ERROR: reqwest error / connection refused / 429 Too Many Requests
```

**Fix**: Check RPC endpoint availability. For EVM chains, the relay polls in 10-block ranges to respect rate limits.

**4. Relay out of gas**

The relay's operator wallet needs funded on each destination chain to pay for `deliver_message` transactions.

**Fix**: Fund the relay's address on each chain.

## Duplicate Message Handling

```text
WARN: E_ALREADY_DELIVERED / Already delivered (nonce=X)
```

This is **normal and expected**. The relay safely skips already-delivered messages. This can happen after:

- Relay restart (re-processes recent messages)
- Multiple relay instances running
- Manual message delivery during debugging

No action needed.

## EVM-Specific Issues

### Transaction reverts

Check the relay log for the revert reason:

```text
ERROR: EVM deliver_message reverted: {reason}
```

Common revert reasons:

- `"Not authorized"` -- relay address not in authorized list
- `"Unknown remote endpoint"` -- remote GMP endpoint not configured
- `"Already delivered"` -- message already processed (safe to ignore)

### Gas estimation failures

The relay uses a fixed gas limit. If delivery consistently fails with out-of-gas errors, the on-chain logic may have changed.

## SVM-Specific Issues

### Message account not found

```text
WARN: SVM outbox: message account not found for dst_chain=X, nonce=Y
```

This can happen if:

- The message PDA was cleaned up (expired)
- The nonce is ahead of actual messages

The relay advances the cursor past missing messages automatically.

### Transaction signature verification failed

Ensure the relay's Ed25519 keypair matches the authorized relay address on the SVM GMP endpoint program.

## MVM-Specific Issues

### CLI execution failures

The relay calls the Movement CLI (`aptos move run`) for MVM delivery. Common issues:

- CLI not in PATH (use `nix develop` shell)
- Profile not configured (`aptos init`)
- Account not funded on target network

### View function failures

```text
DEBUG: Failed view function call
```

View function failures during polling are usually transient (RPC issues). The relay retries on the next poll cycle.

## Debugging Checklist

1. **Is the relay running?** Check `/health` endpoint
2. **Is the relay authorized?** Check startup logs for authorization checks
3. **Is the relay funded?** Check operator wallet balance on each chain
4. **Are remote endpoints configured?** Check each chain's GMP contract configuration
5. **Do chain IDs match?** Compare relay config, intent params, and on-chain config
6. **Is the RPC reachable?** Test RPC endpoints independently
7. **Check relay logs** for ERROR/WARN messages with the intent_id or nonce
