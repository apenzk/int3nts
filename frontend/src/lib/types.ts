// ============================================================================
// Protocol Types
// ============================================================================

// ============================================================================
// Helpers
// ============================================================================

/**
 * Generate a random 32-byte hex string for intent_id.
 * Matches the format used in e2e tests: "0x" + 64 hex chars
 */
export function generateIntentId(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  const hex = Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
  return `0x${hex}`;
}

// ============================================================================
// Draft Intent Types
// ============================================================================

export interface DraftIntentRequest {
  requester_addr: string;
  draft_data: {
    // Required fields for solver acceptance
    intent_id: string;           // Random 32-byte hex (0x + 64 chars)
    offered_metadata: string;
    offered_amount: string;      // String for large numbers
    offered_chain_id: string;    // Chain ID as string
    desired_metadata: string;
    desired_amount: string;      // String for large numbers
    desired_chain_id: string;    // Chain ID as string
    expiry_time: number;         // Unix timestamp
    issuer: string;              // Requester address
    // Optional flow metadata
    flow_type?: 'inflow' | 'outflow';
  };
  expiry_time: number;
}

export interface DraftIntentResponse {
  draft_id: string;
  status: 'pending' | 'signed' | 'expired';
}

export interface DraftIntentStatus {
  draft_id: string;
  status: 'pending' | 'signed' | 'expired';
  requester_address: string;
  timestamp: number;
  expiry_time: number;
}

export interface DraftIntentSignature {
  signature: string;
  // Hub solver address (MVM) for this draft signature.
  solver_hub_addr: string;
  solver_evm_addr?: string; // Solver's EVM address (for inflow to EVM chains)
  solver_svm_addr?: string; // Solver's SVM address (for inflow to SVM chains)
  timestamp: number;
}

// ============================================================================
// API Response Types
// ============================================================================

export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

// ============================================================================
// Event Types
// ============================================================================

export interface IntentEvent {
  intent_id: string;
  offered_metadata: { inner: string };
  offered_amount: number;
  desired_metadata: { inner: string };
  desired_amount: number;
  revocable: boolean;
  requester_addr: string;
  requester_addr_connected_chain: string | null;
  reserved_solver_addr: string | null;
  connected_chain_id: number | null;
  expiry_time: number;
  timestamp: number;
}

export interface EscrowEvent {
  escrow_id: string;
  intent_id: string;
  offered_metadata: { inner: string };
  offered_amount: number;
  desired_metadata: { inner: string };
  desired_amount: number;
  revocable: boolean;
  requester_addr: string;
  reserved_solver_addr: string | null;
  chain_id: number;
  chain_type: 'Mvm' | 'Evm' | 'Svm';
  expiry_time: number;
  timestamp: number;
}

export interface Approval {
  escrow_id: string;
  intent_id: string;
  signature: string;
  timestamp: number;
}

export interface EventsResponse {
  intent_events: IntentEvent[];
  escrow_events: EscrowEvent[];
  fulfillment_events: any[];
  approvals: Approval[];
}

