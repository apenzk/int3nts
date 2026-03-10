# Coordinator API

Base URL: `http://<host>:<port>` (defaults to `127.0.0.1:3333`)

All responses share the shape:

```json
{
  "success": true|false,
  "message": string,
  "data": <payload|null>
}
```

## GET /health

Health check.

Example

```bash
curl -s http://127.0.0.1:3333/health
```

## GET /events

Returns cached events observed by the monitor (intent, fulfillment).

Response (abbreviated)

```json
{
  "success": true,
  "data": {
    "intent_events": [
      {
        "intent_id": "0x...",
        "offered_metadata": "0x1::test::Token",
        "offered_amount": 0,
        "desired_metadata": "0x1::test::Token",
        "desired_amount": 1000000,
        "revocable": false,
        "requester_addr": "0x...",
        "requester_addr_connected_chain": null,
        "reserved_solver_addr": "0x...",
        "connected_chain_id": null,
        "expiry_time": 2000000,
        "timestamp": 1000000
      }
    ],
    "fulfillment_events": [ { ... } ]
  }
}
```

## GET /acceptance

Returns the exchange rate and fee information for a given token pair. The coordinator looks up the pair in its configured acceptance criteria, then fetches the live exchange rate from the solver.

Query parameters:

- `offered_chain_id` (required) — Chain ID of the offered token
- `offered_token` (required) — Metadata address of the offered token
- `desired_chain_id` (optional) — Chain ID of the desired token (if omitted, returns first match)
- `desired_token` (optional) — Metadata address of the desired token (if omitted, returns first match)

**Response** (200 OK)

```json
{
  "success": true,
  "data": {
    "desired_token": "0x1::test::Token",
    "desired_chain_id": 2,
    "exchange_rate": 1.0,
    "base_fee_in_move": 1000,
    "move_rate": 1.0,
    "fee_bps": 50
  },
  "error": null
}
```

**Example**

```bash
curl "http://127.0.0.1:3333/acceptance?offered_chain_id=1&offered_token=0x1::test::Token&desired_chain_id=2&desired_token=0x1::test::Token"
```

## Negotiation Routing Endpoints

The coordinator provides negotiation routing capabilities for off-chain communication between requesters and solvers. This enables requesters to submit draft intents without needing direct contact with solvers, and allows solvers to discover and sign drafts through a centralized message queue.

**Note**: This is a **polling-based, FCFS (First Come First Served)** system. Solvers poll the coordinator for drafts, and the first solver to submit a valid signature wins.

### POST /draftintent

Submit a draft intent for negotiation. Drafts are open to any solver (no `solver_hub_addr` required).

**Request**

```json
{
  "requester_addr": "0x...",
  "draft_data": {
    "offered_metadata": "...",
    "offered_amount": 1000,
    "desired_metadata": "...",
    "desired_amount": 2000
  },
  "expiry_time": 2000000
}
```

**Response** (200 OK)

```json
{
  "success": true,
  "data": {
    "draft_id": "11111111-1111-1111-1111-111111111111",
    "status": "pending"
  },
  "error": null
}
```

**Example**

```bash
curl -X POST http://127.0.0.1:3333/draftintent \
  -H "Content-Type: application/json" \
  -d '{
    "requester_addr": "0x123...",
    "draft_data": {"offered_metadata": "0x1::test::Token", "offered_amount": 1000},
    "expiry_time": 2000000
  }'
```

### GET /draftintent/:id

Get the status of a specific draft intent.

**Response** (200 OK)

```json
{
  "success": true,
  "data": {
    "draft_id": "11111111-1111-1111-1111-111111111111",
    "status": "pending",
    "requester_addr": "0x123...",
    "timestamp": 1000000,
    "expiry_time": 2000000
  },
  "error": null
}
```

**Status values**: `pending`, `signed`, `expired`

**Example**

```bash
curl http://127.0.0.1:3333/draftintent/11111111-1111-1111-1111-111111111111
```

### GET /draftintents/pending

Get all pending drafts. All solvers see all pending drafts (no filtering). This is a polling endpoint - solvers call this regularly to discover new drafts.

**Response** (200 OK)

```json
{
  "success": true,
  "data": [
    {
      "draft_id": "11111111-1111-1111-1111-111111111111",
      "requester_addr": "0x123...",
      "draft_data": {...},
      "timestamp": 1000000,
      "expiry_time": 2000000
    }
  ],
  "error": null
}
```

**Example**

```bash
curl http://127.0.0.1:3333/draftintents/pending
```

### POST /draftintent/:id/signature

Submit a signature for a draft intent. Implements FCFS logic: first signature wins, later signatures are rejected with 409 Conflict.

**Request**

```json
{
  "solver_hub_addr": "0xabc...",
  "signature": "0x" + "a".repeat(128),
  "public_key": "0x" + "b".repeat(64)
}
```

**Response** (200 OK - first signature)

```json
{
  "success": true,
  "data": {
    "draft_id": "11111111-1111-1111-1111-111111111111",
    "status": "signed"
  },
  "error": null
}
```

**Response** (409 Conflict - draft already signed)

```json
{
  "success": false,
  "data": null,
  "error": "Draft already signed by another solver"
}
```

**Validation**

- Solver must be registered on-chain (verified via `get_solver_public_key`)
- Signature must be Ed25519 format (64 bytes = 128 hex characters)
- Signature must be valid hex

**Example**

```bash
curl -X POST http://127.0.0.1:3333/draftintent/11111111-1111-1111-1111-111111111111/signature \
  -H "Content-Type: application/json" \
  -d '{
    "solver_hub_addr": "0xabc...",
    "signature": "0x'$(python3 -c "print('a'*128)")'",
    "public_key": "0x'$(python3 -c "print('b'*64)")'"
  }'
```

### GET /draftintent/:id/signature

Poll for the signature of a draft intent. Returns the first signature received (FCFS). This is a polling endpoint - requesters call this regularly to check if a signature is available.

**Response** (200 OK - signed)

```json
{
  "success": true,
  "data": {
    "signature": "0x" + "a".repeat(128),
    "solver_hub_addr": "0xabc...",
    "solver_evm_addr": "0xdef...",
    "solver_svm_addr": "So1ver...",
    "timestamp": 1000000
  },
  "error": null
}
```

`solver_evm_addr` and `solver_svm_addr` are optional fields looked up from the on-chain solver registry. They are `null` if the solver has not registered an address for that chain.

**Response** (202 Accepted - pending)

```json
{
  "success": false,
  "data": null,
  "error": "Draft not yet signed"
}
```

**Response** (404 Not Found - draft doesn't exist)

```json
{
  "success": false,
  "data": null,
  "error": "Draft not found"
}
```

**Example**

```bash
curl http://127.0.0.1:3333/draftintent/11111111-1111-1111-1111-111111111111/signature
```
