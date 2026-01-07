'use client';

import { useState } from 'react';
import { verifierClient } from '@/lib/verifier';

export function VerifierTest() {
  const [results, setResults] = useState<string>('');
  const [loading, setLoading] = useState(false);

  const addResult = (label: string, result: any) => {
    setResults((prev) => `${prev}\n[${new Date().toLocaleTimeString()}] ${label}:\n${JSON.stringify(result, null, 2)}\n`);
  };

  const testHealth = async () => {
    setLoading(true);
    try {
      const result = await verifierClient.health();
      addResult('Health Check', result);
    } catch (error) {
      console.error('Health check error:', error);
      addResult('Health Check Error', {
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      });
    } finally {
      setLoading(false);
    }
  };

  const testPublicKey = async () => {
    setLoading(true);
    try {
      const result = await verifierClient.getPublicKey();
      addResult('Public Key', result);
    } catch (error) {
      addResult('Public Key Error', error);
    } finally {
      setLoading(false);
    }
  };

  const testEvents = async () => {
    setLoading(true);
    try {
      const result = await verifierClient.getEvents();
      addResult('Events', result);
    } catch (error) {
      addResult('Events Error', error);
    } finally {
      setLoading(false);
    }
  };

  const testApprovals = async () => {
    setLoading(true);
    try {
      const result = await verifierClient.getApprovals();
      addResult('Approvals', result);
    } catch (error) {
      addResult('Approvals Error', error);
    } finally {
      setLoading(false);
    }
  };

  const testPendingDrafts = async () => {
    setLoading(true);
    try {
      const result = await verifierClient.getPendingDrafts();
      addResult('Pending Drafts', result);
    } catch (error) {
      addResult('Pending Drafts Error', error);
    } finally {
      setLoading(false);
    }
  };

  const verifierUrl = process.env.NEXT_PUBLIC_VERIFIER_URL;

  return (
    <div className="border border-gray-700 rounded p-4 mb-4">
      <h2 className="text-xl font-bold mb-4">Verifier API Test</h2>
      
      {/* Display Verifier URL */}
      <div className="mb-4 p-2 bg-gray-800 rounded text-xs text-gray-400">
        Verifier URL: {verifierUrl || 'Not configured'}
      </div>
      
      <div className="flex flex-wrap gap-2 mb-4">
        <button
          onClick={testHealth}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50"
        >
          Health Check
        </button>
        <button
          onClick={testPublicKey}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50"
        >
          Get Public Key
        </button>
        <button
          onClick={testEvents}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50"
        >
          Get Events
        </button>
        <button
          onClick={testApprovals}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50"
        >
          Get Approvals
        </button>
        <button
          onClick={testPendingDrafts}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50"
        >
          Get Pending Drafts
        </button>
        <button
          onClick={() => setResults('')}
          className="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded text-sm"
        >
          Clear
        </button>
      </div>

      {results && (
        <div className="mt-4">
          <h3 className="text-sm font-bold mb-2">Results:</h3>
          <pre className="bg-gray-900 p-4 rounded text-xs font-mono overflow-auto max-h-96">
            {results}
          </pre>
        </div>
      )}
    </div>
  );
}

