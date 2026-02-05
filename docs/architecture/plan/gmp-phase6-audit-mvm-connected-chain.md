# Phase 6 Audit: MVM Connected Chain Modules

**Status:** Complete
**Date:** 2026-02-05

---

## Executive Summary

The MVM connected chain modules (`inflow_escrow_gmp`, `outflow_validator_impl`) are **cleanly isolated** from hub modules. They only depend on:
1. Standard library (`std`)
2. Aptos framework (`aptos_framework`, `aptos_std`)
3. GMP layer (`gmp_common`, `gmp_sender`)

The **only problematic module** is `native_gmp_endpoint`, which imports both hub and connected chain modules for routing. This is the reason the package split is required.

---

## Module Analysis

### Connected Chain Module: `inflow_escrow_gmp.move`

**Purpose:** Handles inflow escrows when MVM acts as a connected chain.

**Dependencies:**
| Dependency | Type | Required for Connected? |
|------------|------|------------------------|
| `std::signer` | std | Yes |
| `std::vector` | std | Yes |
| `std::bcs` | std | Yes |
| `std::from_bcs` | std | Yes |
| `aptos_framework::event` | framework | Yes |
| `aptos_framework::fungible_asset` | framework | Yes |
| `aptos_framework::object` | framework | Yes |
| `aptos_framework::primary_fungible_store` | framework | Yes |
| `aptos_framework::timestamp` | framework | Yes |
| `aptos_framework::account` | framework | Yes (resource account) |
| `aptos_std::table` | framework | Yes |
| `aptos_std::hash` | framework | Yes |
| `mvmt_intent::gmp_common` | **GMP layer** | Yes |
| `mvmt_intent::gmp_sender` | **GMP layer** | Yes |

**Hub Dependencies:** NONE

---

### Connected Chain Module: `outflow_validator.move` (module: `outflow_validator_impl`)

**Purpose:** Validates solver fulfillments when MVM acts as a connected chain.

**Dependencies:**
| Dependency | Type | Required for Connected? |
|------------|------|------------------------|
| `std::signer` | std | Yes |
| `std::vector` | std | Yes |
| `std::bcs` | std | Yes |
| `std::from_bcs` | std | Yes |
| `aptos_framework::event` | framework | Yes |
| `aptos_framework::fungible_asset::Metadata` | framework | Yes |
| `aptos_framework::object` | framework | Yes |
| `aptos_framework::primary_fungible_store` | framework | Yes |
| `aptos_framework::timestamp` | framework | Yes |
| `aptos_std::table` | framework | Yes |
| `mvmt_intent::gmp_common` | **GMP layer** | Yes |
| `mvmt_intent::gmp_sender` | **GMP layer** | Yes |

**Hub Dependencies:** NONE

---

### GMP Layer: `gmp_common` (messages.move)

**Purpose:** Message encoding/decoding for GMP wire format.

**Dependencies:**
| Dependency | Type | Notes |
|------------|------|-------|
| `std::vector` | std | Only standard library! |

**Hub Dependencies:** NONE
**Connected Chain Dependencies:** NONE

This module is **completely standalone** - ideal base layer.

---

### GMP Layer: `gmp_sender.move`

**Purpose:** Outbound GMP message sending with outbox.

**Dependencies:**
| Dependency | Type | Notes |
|------------|------|-------|
| `std::signer` | std | |
| `aptos_framework::event` | framework | |
| `aptos_framework::table` | framework | Note: `aptos_framework::table`, not `aptos_std::table` |
| `aptos_framework::timestamp` | framework | |

**Hub Dependencies:** NONE
**Connected Chain Dependencies:** NONE

Clean GMP layer module.

---

### Problem Module: `native_gmp_endpoint.move`

**Purpose:** Inbound GMP message routing.

**Dependencies (PROBLEMATIC):**
| Dependency | Type | Problem |
|------------|------|---------|
| `mvmt_intent::gmp_common` | GMP layer | OK |
| `mvmt_intent::intent_gmp_hub` | **HUB** | Creates forced hub dependency |
| `mvmt_intent::outflow_validator_impl` | Connected | OK for connected |
| `mvmt_intent::inflow_escrow_gmp` | Connected | OK for connected |

**Issue:** The `route_message()` function imports ALL handlers (hub + connected), creating a forced dependency between them.

**Current Routing Logic:**
```move
if (msg_type == MESSAGE_TYPE_INTENT_REQUIREMENTS) {
    outflow_validator_impl::receive_intent_requirements(...);
    inflow_escrow_gmp::receive_intent_requirements(...);
} else if (msg_type == MESSAGE_TYPE_ESCROW_CONFIRMATION) {
    intent_gmp_hub::receive_escrow_confirmation(...);  // HUB ONLY
} else if (msg_type == MESSAGE_TYPE_FULFILLMENT_PROOF) {
    if (intent_gmp_hub::is_initialized()) { ... }      // HUB conditional
    if (inflow_escrow_gmp::is_initialized()) { ... }   // Connected conditional
}
```

---

## Recommended Package Structure

### Package 1: `intent_gmp` (Base Layer)

**Deploy to:** Both hub and connected chains
**Size estimate:** ~15-20KB

**Modules:**
- `gmp_common` (messages.move) - Message encoding/decoding
- `gmp_sender` - Outbound message sending

**Note:** `native_gmp_endpoint` base functionality (config, delivery, relay auth) could go here, but routing must be split.

---

### Package 2: `intent_hub` (Hub Only)

**Deploy to:** Hub chain only
**Dependencies:** `intent_gmp`

**Modules:**
- `fa_intent` - Base intent types
- `fa_intent_with_oracle` - Oracle-guarded intents
- `fa_intent_inflow` - Inflow wrapper
- `fa_intent_outflow` - Outflow wrapper
- `intent_gmp_hub` - Hub-side GMP handling
- `solver_registry` - Solver registration
- `intent_registry` - Intent tracking
- `native_gmp_endpoint` (hub version) - Routes to `intent_gmp_hub` only

---

### Package 3: `intent_connected` (Connected Chain Only)

**Deploy to:** Connected MVM chains only
**Dependencies:** `intent_gmp`

**Modules:**
- `intent_outflow_validator` (rename from `outflow_validator_impl`)
- `inflow_escrow_gmp`
- `native_gmp_endpoint` (connected version) - Routes to validators + escrow only

---

## Key Findings

1. **Connected chain modules are clean.** `inflow_escrow_gmp` and `outflow_validator_impl` have ZERO hub dependencies.

2. **GMP layer is clean.** `gmp_common` and `gmp_sender` have ZERO application-level dependencies.

3. **`native_gmp_endpoint` is the coupling point.** Its `route_message()` function imports both hub and connected modules.

4. **Split is straightforward.** After split:
   - Hub package has its own `route_message` (only calls `intent_gmp_hub`)
   - Connected package has its own `route_message` (only calls `outflow_validator_impl` + `inflow_escrow_gmp`)
   - No `is_initialized()` conditionals needed - missing init is a hard failure

5. **Module rename required:** `outflow_validator_impl` should be renamed to `intent_outflow_validator_impl` per Phase 6 naming convention.

---

## File Rename Summary

| Current File | Current Module | New File | New Module |
|--------------|----------------|----------|------------|
| `gmp/outflow_validator.move` | `outflow_validator_impl` | `gmp/intent_outflow_validator.move` | `intent_outflow_validator_impl` |

---

## Size Estimates

Based on current module sizes:

| Package | Estimated Size | Under 60KB? |
|---------|---------------|-------------|
| `intent_gmp` | ~15-20KB | Yes |
| `intent_hub` | ~40-50KB | Yes |
| `intent_connected` | ~25-30KB | Yes |

Current combined package: **108KB** (exceeds 60KB limit)

---

## Next Steps

1. **Commit 2:** Split MVM package into three packages
2. **Commit 3:** Rename SVM and EVM programs for consistency
3. **Commit 4:** Verify dependencies are minimal after split
