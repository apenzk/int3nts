# Coordinator Test Completeness

> **⚠️ IMPORTANT: This file tracks VM-specific tests for the Coordinator service only.**
>
> The coordinator is a read-only service that monitors hub chain events and provides negotiation routing.
> It does NOT perform validation or cryptographic signing - those functions are in the **Integrated GMP** service.

This document tracks test alignment status for the coordinator. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

The coordinator has no VM-specific tests. It monitors only the hub chain (Move VM) and provides chain-agnostic negotiation routing. VM-specific chain client tests are in `chain-clients/` — see [chain-clients extension checklist](../../chain-clients/extension-checklist.md).
