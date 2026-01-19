import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';
import { MvmWalletConnector } from './MvmWalletConnector';

const connectMock = vi.fn();
const disconnectMock = vi.fn();

const mockState = {
  connected: false,
  account: null as { address: string } | null,
  wallets: [] as { name: string }[],
};

vi.mock('@aptos-labs/wallet-adapter-react', () => ({
  useWallet: () => ({
    connected: mockState.connected,
    account: mockState.account,
    wallets: mockState.wallets,
    connect: connectMock,
    disconnect: disconnectMock,
  }),
}));

describe('MvmWalletConnector', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    connectMock.mockClear();
    disconnectMock.mockClear();
    mockState.connected = false;
    mockState.account = null;
    mockState.wallets = [];
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });

  /**
   * Test: Missing wallet adapters
   * Why: UI should disable when no MVM wallet is detected.
   */
  it('should disable when no wallet is detected', async () => {
    render(<MvmWalletConnector />);
    const button = await screen.findByText('MVM');
    expect(button).toBeDisabled();
  });

  /**
   * Test: Connected state rendering
   * Why: Users should be able to disconnect when connected.
   */
  it('should show disconnect when connected', async () => {
    mockState.connected = true;
    mockState.account = { address: '0xabc' };
    render(<MvmWalletConnector />);
    const button = await screen.findByText('Disconnect MVM');
    expect(button).toBeInTheDocument();
  });
});
