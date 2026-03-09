# GMP DKG Analysis

## Summary

This document analyzes approaches for GMP message verification. Section A examines how LZ handles this with independent DVN attestations — this is the chosen approach for int3nts. Section B catalogues threshold signing schemes (FROST, WSTS, BLS, threshold ECDSA, simple multisig) as a reference alternative that we do not plan to implement.

**Approach: LZ-style independent relay model.** Multiple relays are configured on-chain (e.g., 3), with a required verification threshold of (for now) 1 — any single relay can deliver and verify a message. This gives redundancy and liveness (if one relay is down, others can deliver) without requiring coordination or shared keys between relays.

---

## A) LZ DVN Model

LZ V2 uses Decentralized Verifier Networks (DVNs) — independent nodes that each submit separate on-chain attestations. There is no shared key or threshold signature.

### How It Works

1. **Message sent on source chain**: App calls `lzSend()` on the Endpoint. The `SendUln302` MessageLib computes a `payloadHash` and emits `PacketSent` + `DVNFeePaid` events.

2. **DVN nodes watch source chain**: Each DVN independently monitors for assigned jobs, waits for configured block confirmations, and verifies the `payloadHash`.

3. **DVN writes attestation to destination chain**: Each DVN calls `verify()` on the destination chain's `ReceiveUln302` MessageLib. This records an entry in a mapping:

   ```text
   hashLookup[headerHash][payloadHash][dvnAddress] = Verification { submitted: true, confirmations: N }
   ```

   Each DVN submits its own transaction from its own wallet. No cryptographic message signature is produced — the "verification" is that a known DVN address wrote to the mapping.

4. **Quorum check + commit**: Once all required DVNs and enough optional DVNs have attested (configurable per-app as "X of Y of N"), anyone can call `commitVerification()` to finalize the message at the Endpoint.

5. **Executor delivers**: A separate Executor calls `lzReceive()` to deliver the payload to the destination application.

### Security Model

- **Identity-based**: DVN wallet addresses are registered on-chain. Authorization = `msg.sender` is in the DVN list.
- **Redundancy**: Multiple independent DVNs must all agree.
- **Economic**: DVNs are paid per-message fees. The EigenLayer CryptoEconomic DVN Framework adds slashing for misbehavior.
- **Flexible**: Apps configure their own security stack (e.g., LZ Labs DVN as required + 2-of-5 optional DVNs).

### Trade-offs

| Aspect | LZ DVN Model |
|--------|-------------|
| Gas cost per message | N separate transactions (one per DVN) |
| Off-chain coordination | None — DVNs act independently |
| Committee changes | Simple — add/remove addresses from on-chain config |
| On-chain complexity | Quorum counting logic in MessageLib contracts |
| Trust model | Marketplace of independent verifiers |

### Applicability to int3nts

The LZ DVN model fits int3nts well. The gas cost of multiple verification transactions per message is acceptable — Movement's own LZ bridge (e.g., WBTC.e OFT) already operates with `requiredDVNCount: 3` (LZ Labs, Horizen, P2P) and 250,000 block confirmations. The independence of relays (no coordination, no DKG ceremonies, trivial committee changes) is a practical advantage, not a limitation.

**int3nts relay design:** Configure multiple relays on-chain (e.g., 3) with a verification threshold of 1. Any single relay can deliver and verify a message. This provides redundancy and liveness — if one relay is down, others can still deliver — without requiring coordination or shared keys between relays.

---

## B) Threshold Signing Scheme Comparison (Reference Only)

> **Not planned for implementation.** This section is retained as a reference in case threshold signing becomes relevant in the future (e.g., if a decentralized open committee is needed). The current approach uses independent relays with a threshold of 1.

The goal would be to replace individual relay keys with a committee of nodes that collectively produce one signature per message. This requires Distributed Key Generation (DKG) for setup and a threshold signing protocol for ongoing operation.

### Schemes Evaluated

#### FROST Ed25519 (Zcash Foundation)

- **Scheme**: Schnorr threshold signatures over Ed25519 (RFC 9591)
- **Output**: Standard Ed25519 signature, indistinguishable from single-key
- **DKG**: Built-in trusted dealer and full DKG protocol
- **Signing rounds**: 2 rounds
- **Maturity**: Stable, NCC-audited, IETF-standardized
- **Crate**: `frost-ed25519`, `frost-core`
- **Variants**: Also available as `frost-secp256k1`, `frost-p256`, `frost-ristretto255`
- **Committee changes**: Requires new DKG ceremony and on-chain public key update
- **Repository**: https://github.com/ZcashFoundation/frost

#### WSTS — Weighted Schnorr Threshold Signatures (Stacks/sBTC)

- **Scheme**: Weighted FROST — each signer can control multiple key shares
- **Output**: Standard Schnorr signature, indistinguishable from single-key
- **DKG**: Built-in, supports weighted thresholds
- **Maturity**: Production (powers sBTC on Stacks)
- **Crate**: `wsts`
- **Committee changes**: Requires re-keying
- **Repository**: https://github.com/stacks-sbtc/wsts

#### Commonware (BLS12-381)

- **Scheme**: BLS12-381 threshold signatures
- **Output**: BLS signature (requires BLS verification on-chain)
- **DKG**: Full DKG + epoch-based resharing built in
- **Maturity**: Alpha (level 0 of 5), unaudited
- **Extras**: Includes consensus engine, p2p networking, runtime
- **Committee changes**: Built-in resharing at epoch boundaries
- **Repository**: https://github.com/commonwarexyz/monorepo

#### LFDT cggmp21 (Threshold ECDSA)

- **Scheme**: ECDSA threshold (CGGMP21 protocol) over secp256k1
- **Output**: Standard ECDSA signature (native `ecrecover` on EVM)
- **DKG**: Built-in
- **Maturity**: Production, Kudelski-audited
- **Committee changes**: Key refresh (not full resharing)
- **Repository**: https://github.com/LFDT-Lockness/cggmp21
- **Note**: LFDT also maintains `givre`, their own FROST implementation

#### blsful + gennaro-dkg (BLS12-381)

- **Scheme**: BLS12-381 with Shamir secret sharing or Gennaro DKG
- **Output**: BLS signature
- **Maturity**: Audited (Kudelski)
- **Committee changes**: Manual re-keying

#### Simple Multisig (no threshold crypto)

- **Scheme**: Each node has its own key, contract checks K-of-N signatures
- **Output**: N individual signatures verified on-chain
- **DKG**: Not needed — each node generates its own key
- **Committee changes**: Trivial — add/remove keys on-chain
- **Gas cost**: Higher — N signature verifications per message

### Comparison Matrix

| Criteria | FROST Ed25519 | WSTS | Commonware (BLS) | LFDT cggmp21 (ECDSA) | Simple Multisig |
|----------|--------------|------|------------------|---------------------|-----------------|
| Signature output | Ed25519 | Ed25519/Schnorr | BLS12-381 | ECDSA/secp256k1 | N individual sigs |
| MVM verification | Native | Native | Native (bls12381) | Not native | N native verifies |
| EVM verification | Not native | Not native | Requires EIP-2537 | Native (ecrecover) | N ecrecover calls |
| SVM verification | Native | Native | Not native | Not native | N native verifies |
| On-chain gas (per msg) | 1 sig verify | 1 sig verify | 1 sig verify | 1 sig verify | N sig verifies |
| Audited | Yes (NCC) | Production | No | Yes (Kudelski) | N/A |
| Standardized | Yes (RFC 9591) | No | No | No | N/A |
| Committee rotation | Re-key | Re-key | Built-in reshare | Key refresh | Trivial |
| Integration complexity | Medium | Medium | High | Medium-High | Low |
| Dependency footprint | Minimal | Minimal | Large (consensus+p2p) | Moderate | None |

### On-Chain Verification Impact

On-chain verification cost is a key factor:

- **Ed25519**: Natively supported on MVM (`ed25519::signature_verify_strict()`) and SVM. Cheap, single opcode.
- **ECDSA/secp256k1**: Natively supported on EVM (`ecrecover`). Not natively available on MVM or SVM.
- **BLS12-381**: Supported on MVM (`aptos_std::bls12381`). On EVM requires EIP-2537 precompiles (post-Pectra) or expensive pure-Solidity verification. No native support on SVM.

FROST Ed25519 and WSTS produce standard Ed25519 signatures — a single `ed25519_verify` call on any chain that supports it. BLS and ECDSA threshold schemes each have gaps in native VM support.

For EVM, where Ed25519 is not natively available, two paths exist: (a) use the `frost-secp256k1` variant for EVM-native `ecrecover`, or (b) use `msg.sender` authorization where the submitting relay is whitelisted, with the threshold signature covering the message payload verified on other VMs.

### If Threshold Signing Were Needed

**FROST Ed25519** would be the strongest candidate:

1. **Simple on-chain verification** — produces a standard Ed25519 signature, verified with a single native call on MVM and SVM
2. **Audited and standardized** — RFC 9591, NCC audit, actively maintained
3. **Minimal dependency footprint** — `frost-ed25519` and `frost-core` only, no consensus or p2p stack
4. **EVM path exists** — `frost-secp256k1` for native `ecrecover`, or relay authorization for gas submission

WSTS would be an alternative if weighted thresholds were needed. Commonware worth revisiting once it matures past Alpha.
