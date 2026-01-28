# Intent Framework Documentation

## Overview

A framework for creating programmable intents. Supports single-chain intents (unreserved, reserved, oracle-guarded) and cross-chain intents (inflow with escrows, outflow with transfers). For cross-chain operations, a coordinator service monitors chains and a trusted-gmp service provides approval signatures.

## Getting Started

- **[Protocol overview](protocol.md)** - Cross-chain intent system flows and sequence diagrams
- **[Documentation Guide](docs-guide.md)** - Documentation structure and navigation
- **[Framework Extension Guide](intent-frameworks/framework-extension-guide.md)** - How to add new blockchain frameworks while maintaining test alignment

## Components

- **[Intent Frameworks](intent-frameworks/README.md)** - Move, EVM, and SVM intent frameworks
- **[Coordinator](coordinator/README.md)** - Read-only event monitoring and negotiation service
- **[Trusted GMP](trusted-gmp/README.md)** - Cross-chain validation and signing service
- **[Solver Tools](solver/README.md)** - Solver service and tools for automatic signature generation and transaction templates
- **[Testing Infrastructure](testing-infra/README.md)** - Chain setup and testing infrastructure
- **[Frontend](frontend/README.md)** - Next.js web interface for intent creation and tracking
