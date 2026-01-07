// Protocol types for cross-chain intents

export interface DraftIntentRequest {
  requester_addr: string;
  draft_data: {
    offered_metadata: string;
    offered_amount: number;
    desired_metadata: string;
    desired_amount: number;
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
  solver_addr: string;
  timestamp: number;
}

export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

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
  chain_type: 'Mvm' | 'Evm';
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

