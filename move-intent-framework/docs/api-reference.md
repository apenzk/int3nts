# API Reference

This document provides a comprehensive reference for the Intent Framework's public APIs.

## Core Intent API

### Creating an Intent

```move
public fun create_intent<Source: store, Args: store + drop, Witness: drop>(
    offered_resource: Source,
    argument: Args,
    expiry_time: u64,
    issuer: address,
    _witness: Witness,
): Object<TradeIntent<Source, Args>>
```

**Parameters:**
- `offered_resource`: The resource being offered in the trade
- `argument`: Trade-specific arguments (e.g., wanted asset type and amount)
- `expiry_time`: Unix timestamp when the intent expires
- `issuer`: Address of the intent creator
- `_witness`: Type witness for compile-time verification

**Returns:** An object containing the trade intent

### Starting a Trading Session

```move
public fun start_intent_session<Source: store, Args: store + drop>(
    intent: Object<TradeIntent<Source, Args>>,
): (Source, TradeSession<Args>)
```

**Parameters:**
- `intent`: The trade intent object

**Returns:** A tuple containing the offered resource and a trading session

### Completing an Intent

```move
public fun finish_intent_session<Witness: drop, Args: store + drop>(
    session: TradeSession<Args>,
    _witness: Witness,
)
```

**Parameters:**
- `session`: The active trading session
- `_witness`: Verification witness proving trade conditions were met

## Fungible Asset Intent API

### Creating a Fungible Asset Intent

```move
public fun create_fa_to_fa_intent_entry(
    source_metadata: Object<Metadata>,
    source_amount: u64,
    desired_metadata: Object<Metadata>,
    desired_amount: u64,
    expiry_time: u64,
    solver_address: address,
    solver_signature: vector<u8>,
): Object<TradeIntent<FungibleAsset, FungibleAssetLimitOrder>>
```

**Parameters:**
- `source_metadata`: Metadata of the asset being offered
- `source_amount`: Amount of the source asset
- `desired_metadata`: Metadata of the desired asset
- `desired_amount`: Amount of the desired asset
- `expiry_time`: Unix timestamp when the intent expires
- `solver_address`: Address of the authorized solver (0x0 for unreserved)
- `solver_signature`: Solver's signature (empty vector for unreserved)

**Returns:** A fungible asset trade intent object

### Starting a Fungible Asset Session

```move
public fun start_fa_offering_session(
    intent: Object<TradeIntent<FungibleAsset, FungibleAssetLimitOrder>>,
): (FungibleAsset, TradeSession<FungibleAssetLimitOrder>)
```

**Parameters:**
- `intent`: The fungible asset trade intent

**Returns:** The offered fungible asset and trading session

### Completing a Fungible Asset Intent

```move
public fun finish_fa_receiving_session(
    session: TradeSession<FungibleAssetLimitOrder>,
    payment: FungibleAsset,
): FungibleAsset
```

**Parameters:**
- `session`: The active trading session
- `payment`: The fungible asset being provided as payment

**Returns:** The fungible asset received in exchange

## Intent Reservation API

### Creating a Draft Intent

```move
public fun create_draft_intent(
    source_metadata: Object<Metadata>,
    source_amount: u64,
    desired_metadata: Object<Metadata>,
    desired_amount: u64,
    expiry_time: u64,
    offerer_address: address,
): IntentDraft
```

**Parameters:**
- `source_metadata`: Metadata of the asset being offered
- `source_amount`: Amount of the source asset
- `desired_metadata`: Metadata of the desired asset
- `desired_amount`: Amount of the desired asset
- `expiry_time`: Unix timestamp when the intent expires
- `offerer_address`: Address of the intent creator

**Returns:** A draft intent for off-chain sharing

### Adding Solver to Draft

```move
public fun add_solver_to_draft_intent(
    draft: IntentDraft,
    solver_address: address,
): IntentToSign
```

**Parameters:**
- `draft`: The draft intent
- `solver_address`: Address of the solver

**Returns:** Intent data ready for signing

### Verifying and Creating Reservation

```move
public fun verify_and_create_reservation(
    intent_to_sign: IntentToSign,
    solver_signature: vector<u8>,
    solver_address: address,
): Option<IntentReserved>
```

**Parameters:**
- `intent_to_sign`: The intent data that was signed
- `solver_signature`: The solver's signature
- `solver_address`: Address of the solver

**Returns:** An optional reservation if verification succeeds

## Events

### LimitOrderEvent

Emitted when a fungible asset intent is created:

```move
struct LimitOrderEvent has drop, store {
    intent_id: Object<TradeIntent<FungibleAsset, FungibleAssetLimitOrder>>,
    source_metadata: Object<Metadata>,
    source_amount: u64,
    desired_metadata: Object<Metadata>,
    desired_amount: u64,
    expiry_time: u64,
    offerer: address,
    solver: address,
}
```

## Error Codes

- `EINVALID_SIGNATURE`: Signature verification failed
- `EINTENT_EXPIRED`: Intent has passed its expiry time
- `EUNAUTHORIZED_SOLVER`: Attempted execution by unauthorized solver
- `EINVALID_AMOUNT`: Invalid asset amount specified
- `EINVALID_METADATA`: Invalid asset metadata provided

## Type Definitions

### TradeIntent

```move
struct TradeIntent<Source: store, Args: store + drop> has key {
    offered_resource: Source,
    argument: Args,
    expiry_time: u64,
    issuer: address,
    reservation: Option<IntentReserved>,
}
```

### TradeSession

```move
struct TradeSession<Args: store + drop> has store {
    argument: Args,
    expiry_time: u64,
    issuer: address,
}
```

### FungibleAssetLimitOrder

```move
struct FungibleAssetLimitOrder has store, drop {
    wanted_metadata: Object<Metadata>,
    wanted_amount: u64,
}
```
