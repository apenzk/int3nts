// Verifier API client with polling support

import type {
  ApiResponse,
  DraftIntentRequest,
  DraftIntentResponse,
  DraftIntentStatus,
  DraftIntentSignature,
  EventsResponse,
  Approval,
} from './types';

const VERIFIER_URL = process.env.NEXT_PUBLIC_VERIFIER_URL;
if (!VERIFIER_URL) {
  throw new Error('NEXT_PUBLIC_VERIFIER_URL environment variable is not set');
}

class VerifierClient {
  private baseUrl: string;

  constructor(baseUrl: string = VERIFIER_URL) {
    this.baseUrl = baseUrl;
  }

  private async fetch<T>(
    endpoint: string,
    options?: RequestInit
  ): Promise<ApiResponse<T>> {
    try {
      const response = await fetch(`${this.baseUrl}${endpoint}`, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          ...options?.headers,
        },
      });

      if (!response.ok) {
        let errorMessage: string;
        try {
          const error = await response.json();
          if (error.error) {
            errorMessage = error.error;
          } else {
            errorMessage = `HTTP ${response.status}: ${response.statusText}`;
          }
        } catch {
          errorMessage = `HTTP ${response.status}: ${response.statusText}`;
        }
        return {
          success: false,
          data: null,
          error: errorMessage,
        };
      }

      const data = await response.json();
      return data as ApiResponse<T>;
    } catch (error) {
      if (!(error instanceof Error)) {
        throw new Error('Unknown error occurred');
      }
      const errorMessage = error.message;
      const detailedError = error instanceof TypeError && errorMessage.includes('fetch')
        ? `Failed to connect to verifier at ${this.baseUrl}. Is it running?`
        : errorMessage;
      
      return {
        success: false,
        data: null,
        error: detailedError,
      };
    }
  }

  // Health check
  async health(): Promise<ApiResponse<string>> {
    return this.fetch<string>('/health');
  }

  // Get public key
  async getPublicKey(): Promise<ApiResponse<{ public_key: string }>> {
    return this.fetch<{ public_key: string }>('/public-key');
  }

  // Draft intent endpoints
  async createDraftIntent(
    request: DraftIntentRequest
  ): Promise<ApiResponse<DraftIntentResponse>> {
    return this.fetch<DraftIntentResponse>('/draftintent', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  }

  async getDraftIntentStatus(
    draftId: string
  ): Promise<ApiResponse<DraftIntentStatus>> {
    return this.fetch<DraftIntentStatus>(`/draftintent/${draftId}`);
  }

  async getPendingDrafts(): Promise<ApiResponse<DraftIntentStatus[]>> {
    return this.fetch<DraftIntentStatus[]>('/draftintents/pending');
  }

  async submitDraftSignature(
    draftId: string,
    solverAddr: string,
    signature: string,
    publicKey: string
  ): Promise<ApiResponse<DraftIntentResponse>> {
    return this.fetch<DraftIntentResponse>(`/draftintent/${draftId}/signature`, {
      method: 'POST',
      body: JSON.stringify({
        solver_addr: solverAddr,
        signature,
        public_key: publicKey,
      }),
    });
  }

  // Poll for draft signature (returns 202 if pending, 200 if signed)
  async pollDraftSignature(
    draftId: string
  ): Promise<ApiResponse<DraftIntentSignature>> {
    return this.fetch<DraftIntentSignature>(
      `/draftintent/${draftId}/signature`
    );
  }

  // Events and approvals
  async getEvents(): Promise<ApiResponse<EventsResponse>> {
    return this.fetch<EventsResponse>('/events');
  }

  async getApprovals(): Promise<ApiResponse<Approval[]>> {
    return this.fetch<Approval[]>('/approvals');
  }

  async getApprovalByEscrow(escrowId: string): Promise<ApiResponse<Approval>> {
    return this.fetch<Approval>(`/approvals/${escrowId}`);
  }

  // Validation endpoints
  async validateOutflowFulfillment(
    transactionHash: string,
    chainType: 'mvm' | 'evm',
    intentId?: string
  ): Promise<
    ApiResponse<{
      validation: {
        valid: boolean;
        message: string;
        timestamp: number;
      };
      approval_signature: {
        signature: string;
        timestamp: number;
      };
    }>
  > {
    return this.fetch(`/validate-outflow-fulfillment`, {
      method: 'POST',
      body: JSON.stringify({
        transaction_hash: transactionHash,
        chain_type: chainType,
        intent_id: intentId,
      }),
    });
  }

  // Polling utilities
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

  async pollUntilApproval(
    escrowId: string,
    options: {
      interval: number;
      timeout: number;
      onProgress?: (attempt: number) => void;
    }
  ): Promise<ApiResponse<Approval>> {
    const { interval, timeout, onProgress } = options;
    const startTime = Date.now();
    let attempt = 0;

    while (Date.now() - startTime < timeout) {
      attempt++;
      onProgress?.(attempt);

      const response = await this.getApprovalByEscrow(escrowId);

      if (response.success && response.data) {
        return response;
      }

      // If not found, continue polling
      if (response.error?.includes('not found')) {
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

export const verifierClient = new VerifierClient();

