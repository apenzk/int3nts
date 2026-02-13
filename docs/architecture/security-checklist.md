# Security Hardening Checklist

This checklist provides a comprehensive security review guide for the Intent Framework. Each area should be reviewed and hardened before production deployment.

**Total Estimated Time: ~1.5 weeks**

---

## Overview

| # | Area | Time Est | Priority |
|---|------|----------|----------|
| 1 | Endpoint Abuse Prevention | 1.5 days | High |
| 2 | Client Trust Elimination | 1 day | High |
| 3 | Auth Hardening | 1.5 days | High |
| 4 | Logging Infrastructure | 1.5 days | Medium |
| 5 | 3rd Party Resilience | 1 day | Medium |
| 6 | Secrets Management | 0.5 day | High |
| 7 | Breach Response Plan | 1 day | Medium |

---

## 1. Endpoint Abuse Prevention

**Time: 1.5 days** | **Components: Coordinator API, Solver endpoints**

Assume every endpoint will be abused. Attackers don't follow happy paths.

### Requirements

- [ ] **Rate Limiting**: Implement rate limits on all public endpoints
  - Per-IP rate limiting
  - Per-user rate limiting (authenticated endpoints)
  - Burst limits for sudden traffic spikes
  
- [ ] **Idempotency for Writes**: All write operations must be idempotent
  - Use idempotency keys for intent creation
  - Prevent duplicate escrow creation
  - Handle replay of fulfillment requests
  
- [ ] **Server-Side Validation Only**: Never trust client-side validation
  - Validate all inputs on server
  - Check parameter bounds and types
  - Sanitize all user-provided data

### Components to Review

| Component | File/Module | Checks |
|-----------|-------------|--------|
| Coordinator API | `coordinator/src/api/` | Rate limits, input validation |
| Draft Intent Endpoint | `POST /draftintent` | Idempotency, rate limiting |
| Signature Endpoint | `POST /draftintent/:id/signature` | FCFS protection, replay prevention |

---

## 2. Client Trust Elimination

**Time: 1 day** | **Components: Contracts, Integrated GMP relay**

Frontend checks are for UX, not security. All security checks must be server-side.

### Requirements

- [ ] **Server-Side Permission Checks**: All authorization on server
  - Verify solver authorization before processing
  - Check intent ownership for cancellation
  - Validate escrow ownership for claims
  
- [ ] **Server-Side Ownership Validation**: Never trust client claims
  - Verify signer matches expected address
  - Check on-chain state for ownership
  - Validate signatures cryptographically

### Anti-Patterns to Eliminate

```text
❌ "The button is hidden" - not a security strategy
❌ "Frontend validates the input" - bots ignore this
❌ "Only authorized users see this page" - URL is guessable
```

### Components to Review

| Component | Checks |
|-----------|--------|
| Move Contracts | `signer` verification, ownership checks |
| GMP Endpoint Contracts | Relay authorization, remote endpoint verification |
| Solver | Transaction signing, permission checks |

---

## 3. Auth Hardening

**Time: 1.5 days** | **Components: GMP endpoint auth, relay authorization**

Auth working once doesn't mean auth is safe. Test edge cases.

### Test Scenarios

- [ ] **Concurrent Sessions**: Login twice quickly
- [ ] **Token Refresh**: Refresh mid-request
- [ ] **Token Reuse**: Attempt to reuse old/expired tokens
- [ ] **Out-of-Order Calls**: Call endpoints in unexpected order
- [ ] **Multi-Tab Behavior**: Multiple browser tabs with different states
- [ ] **Session Expiry**: Actions during session timeout

### GMP Message Authentication Hardening

- [ ] Verify relay is authorized on GMP endpoint before delivering messages
- [ ] Check remote GMP endpoint address matches expected source
- [ ] Prevent message replay across different intents (idempotency)
- [ ] Validate GMP message payload covers all relevant fields

### Components to Review

| Component | File | Checks |
|-----------|------|--------|
| Solver Registry | `intent-frameworks/mvm/intent-hub/sources/solver_registry.move` | Public key management |
| GMP Endpoint | `intent-frameworks/mvm/intent-gmp/sources/gmp/intent_gmp.move` | Relay authorization, remote endpoint verification |
| Intent Creation | `create_inflow_intent`, `create_outflow_intent` | Solver signature verification |

---

## 4. Logging Infrastructure

**Time: 1.5 days** | **Components: Integrated GMP relay, Solver**

No logs means no answers. Not for bugs, not for breaches, not for refunds.

### Requirements

- [ ] **Structured Logging**: Use consistent log format

  ```json
  {
    "timestamp": "2026-01-13T10:00:00Z",
    "level": "INFO",
    "user_id": "0x...",
    "request_id": "uuid",
    "action": "intent_fulfilled",
    "intent_id": "0x...",
    "source_ip": "..."
  }
  ```

- [ ] **Sensitive Action Logging**: Log all critical operations
  - Intent creation/fulfillment
  - Escrow creation/claim/refund
  - GMP message delivery
  - Validation results
  - Configuration changes

- [ ] **Correlation IDs**: Track requests across services
  - Generate request_id at entry point
  - Propagate through all service calls
  - Include in all related log entries

### Log Retention

| Log Type | Retention | Purpose |
|----------|-----------|---------|
| Security events | 1 year | Audit, compliance |
| Transaction logs | 6 months | Debugging, disputes |
| Debug logs | 7 days | Development |

---

## 5. 3rd Party Resilience

**Time: 1 day** | **Components: Chain RPC calls, GMP providers**

Third-party services will fail. Design for it.

### Requirements

- [ ] **Retries with Limits**: Implement exponential backoff

  ```text
  Max retries: 3-5
  Initial delay: 100ms
  Max delay: 10s
  Backoff factor: 2x
  ```

- [ ] **Graceful Fallbacks**: Handle service unavailability
  - Multiple RPC endpoints per chain
  - Fallback verification methods
  - Cached state for read operations

- [ ] **No Single-Request-Does-Everything**: Break complex flows
  - Avoid atomic multi-chain operations
  - Use checkpoints and resumable flows
  - Handle partial failures gracefully

### Failure Scenarios to Handle

| Service | Failure Mode | Mitigation |
|---------|--------------|------------|
| Chain RPC | Timeout, rate limit | Multiple providers, caching |
| GMP Provider | Message delay | Timeout handling, retry |
| Integrated GMP relay | Unavailable | Queue pending messages |

---

## 6. Secrets Management

**Time: 0.5 day** | **Components: All**

API keys in code will leak. Not maybe. Will.

### Requirements

- [ ] **Environment Variables**: Use `.env` files
  - Never commit secrets to git
  - Use `.env.example` for documentation
  - Different secrets per environment

- [ ] **Proper .gitignore**: Exclude sensitive files

  ```text
  .env
  .env.local
  .env.*.local
  *.pem
  *.key
  config/secrets/
  ```

- [ ] **Server-Side Only**: Never expose secrets to client
  - No API keys in frontend code
  - No private keys in browser
  - No secrets in client-side config

- [ ] **Key Rotation Procedures**: Document and practice
  - How to rotate each key type
  - Automation where possible
  - Zero-downtime rotation

### Secrets Inventory

| Secret | Location | Rotation Frequency |
|--------|----------|-------------------|
| Integrated GMP operator wallet key | `.env` | Quarterly |
| Chain RPC API keys | `.env` | On compromise |
| Solver private keys | Secure storage | As needed |

---

## 7. Breach Response Plan

**Time: 1 day** | **Components: Documentation + tooling**

The question is not if but when. Be prepared.

### Immediate Response Capabilities

- [ ] **Fast Access Revocation**
  - Disable compromised API keys instantly
  - Revoke solver authorizations
  - Pause contract operations (if pausable)

- [ ] **Key Rotation Under Pressure**
  - Document rotation steps for each key
  - Have backup keys pre-generated
  - Test rotation in non-production

- [ ] **Session Invalidation**
  - Force logout all sessions
  - Invalidate all tokens
  - Require re-authentication

### Communication Plan

- [ ] **User Notification Template**: Pre-written incident disclosure
- [ ] **Internal Escalation Path**: Who to contact, in what order
- [ ] **Public Communication**: Status page, social media

### Incident Response Runbook

| Step | Action | Owner |
|------|--------|-------|
| 1 | Identify scope of breach | Security |
| 2 | Contain - revoke access | Engineering |
| 3 | Preserve evidence | Security |
| 4 | Rotate compromised secrets | Engineering |
| 5 | Assess user impact | Product |
| 6 | Notify affected users | Communications |
| 7 | Post-mortem | All |

---

## Review Schedule

| Phase | Timing | Focus |
|-------|--------|-------|
| Pre-Testnet | Before public testnet | All critical items |
| Pre-Mainnet | Before production | Full checklist |
| Quarterly | Ongoing | New code, dependencies |
| Post-Incident | After any security event | Affected areas |

---

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Smart Contract Security Best Practices](https://consensys.github.io/smart-contract-best-practices/)
- [Move Security Guidelines](https://move-language.github.io/move/)
