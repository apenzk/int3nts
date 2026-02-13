# Intent Framework Documentation

## Overview

A framework for creating programmable intents. Supports single-chain intents (unreserved, reserved, oracle-guarded) and cross-chain intents (inflow with escrows, outflow with transfers). For cross-chain operations, a coordinator service monitors chains and an integrated-gmp relay delivers GMP messages between chains for on-chain validation.

## Getting Started

- **[Protocol overview](protocol.md)** - Cross-chain intent system flows and sequence diagrams
- **[Documentation Guide](docs-guide.md)** - Documentation structure and navigation
- **[Framework Extension Guide](intent-frameworks/framework-extension-guide.md)** - How to add new blockchain frameworks while maintaining test alignment

## Components

- **[Intent Frameworks](intent-frameworks/README.md)** - Move, EVM, and SVM intent frameworks
- **[Coordinator](coordinator/README.md)** - Read-only event monitoring and negotiation service
- **[Integrated GMP](integrated-gmp/README.md)** - GMP message relay for cross-chain communication
- **[Solver Tools](solver/README.md)** - Automated solver service for intent fulfillment
- **[Testing Infrastructure](testing-infra/README.md)** - Chain setup and testing infrastructure
- **[Frontend](frontend/README.md)** - Next.js web interface for intent creation and tracking
