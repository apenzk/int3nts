// ============================================================================
// Coordinator API Client
// ============================================================================

import type {
  ApiResponse,
  DraftIntentRequest,
  DraftIntentResponse,
  DraftIntentStatus,
  DraftIntentSignature,
  EventsResponse,
} from './types';

// ============================================================================
// Configuration
// ============================================================================

const COORDINATOR_URL = process.env.NEXT_PUBLIC_COORDINATOR_URL as string;
if (!COORDINATOR_URL) {
  throw new Error('NEXT_PUBLIC_COORDINATOR_URL environment variable is not set');
}

// ============================================================================
// Client Implementation
// ============================================================================

class CoordinatorClient {
  private coordinatorUrl: string;

  constructor(coordinatorUrl: string = COORDINATOR_URL) {
    this.coordinatorUrl = coordinatorUrl;
  }

  private async fetchFrom<T>(
    baseUrl: string,
    endpoint: string,
    options?: RequestInit
  ): Promise<ApiResponse<T>> {
    try {
      const response = await fetch(`${baseUrl}${endpoint}`, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          ...options?.headers,
        },
      });

      // Parse JSON response regardless of status code
      // The coordinator returns 200 OK for signed, 202 Accepted for pending
      // Both are valid responses and should be parsed
      const data = await response.json();

      // If status is not ok (outside 200-299), treat as error
      if (!response.ok) {
        return {
          success: false,
          data: null,
          error: data.error || `HTTP ${response.status}: ${response.statusText}`,
        };
      }

      return data as ApiResponse<T>;
    } catch (error) {
      if (!(error instanceof Error)) {
        throw new Error('Unknown error occurred');
      }
      const errorMessage = error.message;
      const detailedError = error instanceof TypeError && errorMessage.includes('fetch')
        ? `Failed to connect to service at ${baseUrl}. Is it running?`
        : errorMessage;

      return {
        success: false,
        data: null,
        error: detailedError,
      };
    }
  }

  // --------------------------------------------------------------------------
  // Coordinator endpoints (negotiation, events, exchange rates)
  // --------------------------------------------------------------------------

  // Health check
  async health(): Promise<ApiResponse<string>> {
    return this.fetchFrom<string>(this.coordinatorUrl, '/health');
  }

  // Draft intent endpoints
  async createDraftIntent(
    request: DraftIntentRequest
  ): Promise<ApiResponse<DraftIntentResponse>> {
    return this.fetchFrom<DraftIntentResponse>(this.coordinatorUrl, '/draftintent', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async getDraftIntentStatus(
    draftId: string
  ): Promise<ApiResponse<DraftIntentStatus>> {
    return this.fetchFrom<DraftIntentStatus>(this.coordinatorUrl, `/draftintent/${draftId}`);
  }

  async getPendingDrafts(): Promise<ApiResponse<DraftIntentStatus[]>> {
    return this.fetchFrom<DraftIntentStatus[]>(this.coordinatorUrl, '/draftintents/pending');
  }

  async submitDraftSignature(
    draftId: string,
    solverAddr: string,
    signature: string,
    publicKey: string
  ): Promise<ApiResponse<DraftIntentResponse>> {
    return this.fetchFrom<DraftIntentResponse>(this.coordinatorUrl, `/draftintent/${draftId}/signature`, {
      method: 'POST',
      body: JSON.stringify({
        solver_hub_addr: solverAddr,
        signature,
        public_key: publicKey,
      }),
    });
  }

  // Poll for draft signature (returns 202 if pending, 200 if signed)
  async pollDraftSignature(
    draftId: string
  ): Promise<ApiResponse<DraftIntentSignature>> {
    return this.fetchFrom<DraftIntentSignature>(
      this.coordinatorUrl,
      `/draftintent/${draftId}/signature`
    );
  }

  // Events
  async getEvents(): Promise<ApiResponse<EventsResponse>> {
    return this.fetchFrom<EventsResponse>(this.coordinatorUrl, '/events');
  }

  // Get exchange rate for token pair
  async getExchangeRate(
    offeredChainId: number,
    offeredToken: string,
    desiredChainId?: number,
    desiredToken?: string
  ): Promise<ApiResponse<{
    desired_token: string;
    desired_chain_id: number;
    exchange_rate: number;
    base_fee_in_move: number;
    move_rate: number;
    fee_bps: number;
  }>> {
    const params = new URLSearchParams({
      offered_chain_id: offeredChainId.toString(),
      offered_token: offeredToken,
    });
    if (desiredChainId !== undefined && desiredToken !== undefined) {
      params.append('desired_chain_id', desiredChainId.toString());
      params.append('desired_token', desiredToken);
    }
    return this.fetchFrom(this.coordinatorUrl, `/acceptance?${params.toString()}`);
  }

  // --------------------------------------------------------------------------
  // Polling utilities
  // --------------------------------------------------------------------------

  async pollUntilSigned(
    draftId: string,
    options: {
      interval: number;
      timeout: number;
      onProgress?: (attempt: number) => void;
    }
  ): Promise<ApiResponse<DraftIntentSignature>> {
    const { interval, timeout, onProgress } = options;
    const startTime = Date.now();
    let attempt = 0;

    while (Date.now() - startTime < timeout) {
      attempt++;
      onProgress?.(attempt);

      const response = await this.pollDraftSignature(draftId);

      if (response.success && response.data) {
        return response;
      }

      // If error is "Draft not yet signed", continue polling
      if (response.error?.includes('not yet signed')) {
        await new Promise((resolve) => setTimeout(resolve, interval));
        continue;
      }

      // Other errors, return immediately
      return response;
    }

    return {
      success: false,
      data: null,
      error: 'Polling timeout',
    };
  }

}

export const coordinatorClient = new CoordinatorClient();
