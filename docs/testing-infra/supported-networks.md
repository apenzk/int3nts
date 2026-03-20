# Supported Networks

Networks supported for int3nts contract deployment, with cost estimates.

Gas prices fluctuate constantly. Cost figures are order-of-magnitude estimates as of 2026-03-16, not quotes. Verify current prices before deploying.

## Testnet

| Chain | Supported | Notes |
|---|---|---|
| Base (Sepolia) | yes | |
| Ethereum (Sepolia) | yes | |
| HyperEVM (Hyperliquid Testnet) | no | Testnet tokens (HYPE) not easily obtainable via public faucet |
| Movement (Bardock) | yes | |
| Solana (Devnet) | yes | |

Testnet deployments use faucet tokens and have no real cost.

## Mainnet

| Chain | Supported | Notes |
|---|---|---|
| Base | yes | L2, very cheap |
| Ethereum | no | High gas costs: ~$272 deployment, ~$1-5 per solver/relay tx |
| HyperEVM | yes | Estimated ~$1 |
| Movement | yes | Effectively free |
| Solana | no | High program rent: ~$297 for 3 programs |

## Deployment Cost Breakdown

### Token prices (as of 2026-03-16)

| Chain | Native token | Token price | Deployment cost (USD) |
|---|---|---|---|
| Base mainnet | ETH | $2,270 | < $1 |
| Movement mainnet | MOVE | $0.02 | < $0.01 |
| HyperEVM mainnet | HYPE | $39 | ~$1.17 (guess) |
| Ethereum mainnet | ETH | $2,270 | ~$272 |
| Solana mainnet | SOL | $93 | ~$297 (3 programs, non-upgradeable) |

### EVM chains (3 contracts: IntentGmp, IntentInflowEscrow, IntentOutflowValidator)

All three contracts deploy with the same bytecode on every EVM chain. Total gas: ~6M gas (roughly 2M per contract).

| Chain | Gas price | Native cost | USD cost |
|---|---|---|---|
| Base mainnet (L2) | ~0.005 gwei + blob fees | ~0.0001 ETH | < $1 |
| HyperEVM mainnet | unknown (own L1, paid in HYPE) | ~0.03 HYPE (guess) | ~$1.17 (guess) |
| Ethereum mainnet | ~20 gwei | ~0.12 ETH | ~$272 |

Ethereum mainnet is expensive because block space demand drives gas price up. L2s and alt-L1s execute the same bytecode for a fraction of the cost.

Each EVM chain also requires 2-3 admin transactions (setRemoteGmpEndpointAddr, updateHubConfig). These are cheap calls (~50-100k gas each), adding < $10 on Ethereum mainnet and negligible on L2s.

### Movement mainnet (Move module publishing)

Move module publishing on Movement/Aptos is cheap. The hub intent framework is a single module publish transaction.

| Item | Gas | Native cost | USD cost |
|---|---|---|---|
| Module publish (hub) | ~10k gas units | < 0.1 MOVE | < $0.01 |
| GMP endpoint registration (per chain) | ~1k gas units | < 0.01 MOVE | < $0.01 |

Movement deployment is effectively free.

### Solana mainnet (3 programs: escrow, GMP endpoint, outflow validator)

Included for reference — not currently targeted for mainnet deployment due to cost.

The main cost driver is program binary size. Unlike EVM chains where deployment cost is gas x gas price (a transaction fee), Solana charges rent — a one-time SOL deposit proportional to the on-chain storage the program occupies. Larger binaries = more storage = more SOL locked up. The deposit is permanent (not refunded unless the program is closed) and must be paid upfront.

The rent-exempt minimum is calculated as: `(890,880 + data_size_bytes x 6,960) lamports`. For upgradeable programs, the on-chain `programdata` account stores the full compiled binary plus a 45-byte header, so `data_size_bytes = binary_size + 45`.

Using actual compiled binary sizes from `target/deploy/`:

| Program | Binary size | data_size_bytes | Rent (SOL) | USD |
|---|---|---|---|---|
| intent_inflow_escrow.so | 170,144 bytes | 170,189 bytes | 1.185 SOL | $110 |
| intent_gmp.so | 154,080 bytes | 154,125 bytes | 1.073 SOL | $100 |
| intent_outflow_validator.so | 134,560 bytes | 134,605 bytes | 0.938 SOL | $87 |
| **Total** | **458,784 bytes** | | **3.20 SOL** | **$297** |

If deployed as upgradeable with a 2x size buffer (`--max-len`), the deposit doubles: ~6.4 SOL / ~$594.

Per-intent PDA accounts are tiny (< 0.01 SOL each).
