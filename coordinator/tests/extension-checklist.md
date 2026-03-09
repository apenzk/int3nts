# Coordinator Test Completeness

> **⚠️ IMPORTANT: This file tracks VM-specific tests for the Coordinator service only.**
>
> The coordinator is a read-only service that monitors events and provides negotiation routing.
> It does NOT perform validation or cryptographic signing - those functions are in the **Integrated GMP** service.

This document tracks test alignment status for the coordinator. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## tests/*vm_client_tests.rs (read-only queries)

MVM client tests moved to `chain-clients/mvm/tests/mvm_client_hub_tests.rs`.
SVM client tests moved to `chain-clients/svm/tests/svm_client_tests.rs`.
See [chain-clients extension checklist](../../chain-clients/extension-checklist.md).

## tests/readiness_*vm_tests.rs (IntentRequirementsReceived monitoring)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_poll_*vm_requirements_received_parses_event | ✅ | ✅ | ✅ |
| 2 | test_poll_*vm_requirements_received_handles_empty_events | ✅ | ✅ | ✅ |
| 3 | test_poll_*vm_requirements_received_handles_multiple_events | ✅ | ✅ | ✅ |
| 4 | test_poll_*vm_requirements_received_normalizes_intent_id | ✅ | ✅ | ✅ |
