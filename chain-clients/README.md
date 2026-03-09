# Chain Clients

Shared RPC client libraries for communicating with hub and connected chains. Used by coordinator, integrated-gmp, and solver.

## Crates

| Crate | Description |
| --- | --- |
| [common](common/) | Chain-agnostic utilities (`normalize_intent_id`) |
| [mvm](mvm/) | Move VM REST client |
| [evm](evm/) | Ethereum JSON-RPC client |
| [svm](svm/) | Solana JSON-RPC client |

## Testing

```bash
nix develop ./nix -c bash -c "./chain-clients/scripts/test.sh"
```

## Test Alignment

See [extension-checklist](extension-checklist.md) for cross-VM test coverage tracking.

For the full extraction plan, see [chain-clients-extraction](../docs/architecture/plan/chain-clients-extraction.md).
