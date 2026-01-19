import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { SvmWalletConnector } from './SvmWalletConnector';

const connectMock = vi.fn();
const disconnectMock = vi.fn();
const selectMock = vi.fn();

const mockState = {
  connected: false,
  wallets: [{ adapter: { name: 'Phantom' } }],
};

vi.mock('@solana/wallet-adapter-react', () => ({
  useWallet: () => ({
    connected: mockState.connected,
    wallets: mockState.wallets,
    connect: connectMock,
    disconnect: disconnectMock,
    select: selectMock,
  }),
}));

describe('SvmWalletConnector', () => {
  beforeEach(() => {
    connectMock.mockClear();
    disconnectMock.mockClear();
    selectMock.mockClear();
    mockState.connected = false;
    mockState.wallets = [{ adapter: { name: 'Phantom' } }];
  });

  /**
   * Test: Disconnected state rendering
   * Why: Users should see a connect CTA when no wallet is connected.
   */
  it('should show connect button when disconnected', async () => {
    render(<SvmWalletConnector />);
    const button = await screen.findByText('Connect SVM');
    expect(button).toBeInTheDocument();
  });

  /**
   * Test: Missing Phantom adapter
   * Why: UI should disable if Phantom adapter is unavailable.
   */
  it('should disable when Phantom adapter is not detected', async () => {
    mockState.wallets = [];
    render(<SvmWalletConnector />);
    const button = await screen.findByText('SVM');
    expect(button).toBeDisabled();
  });

  /**
   * Test: Connect action
   * Why: Clicking connect should select Phantom and call connect().
   */
  it('should call select and connect on click', async () => {
    render(<SvmWalletConnector />);
    const button = await screen.findByText('Connect SVM');
    await userEvent.click(button);
    expect(selectMock).toHaveBeenCalledWith('Phantom');
    expect(connectMock).toHaveBeenCalledTimes(1);
  });

  /**
   * Test: Connected state rendering
   * Why: Users should be able to disconnect when connected.
   */
  it('should show disconnect button when connected', async () => {
    mockState.connected = true;
    render(<SvmWalletConnector />);
    const button = await screen.findByText('Disconnect SVM');
    expect(button).toBeInTheDocument();
  });
});
