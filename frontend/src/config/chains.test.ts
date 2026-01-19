import { describe, expect, it } from 'vitest';
import {
  getChainConfig,
  getChainType,
  getEscrowContractAddress,
  getHubChainConfig,
  getSvmProgramId,
  isHubChain,
} from './chains';

describe('getChainConfig', () => {
  /**
   * Test: SVM chain config lookup
   * Why: UI and helpers depend on RPC + program ID being present.
   */
  it('should return config for svm-devnet', () => {
    const config = getChainConfig('svm-devnet');
    expect(config?.rpcUrl).toBeTruthy();
    expect(config?.svmProgramId).toBeTruthy();
    expect(config?.chainType).toBe('svm');
  });
});

describe('hub chain helpers', () => {
  /**
   * Test: Hub chain identification
   * Why: Hub-specific logic should not depend on chain key strings.
   */
  it('should return the configured hub chain', () => {
    const hub = getHubChainConfig();
    expect(isHubChain(hub.id)).toBe(true);
    expect(hub.chainType).toBe('mvm');
  });
});

describe('getChainType', () => {
  /**
   * Test: Chain type lookup
   * Why: VM-specific logic should be driven by config.
   */
  it('should return evm for base-sepolia', () => {
    expect(getChainType('base-sepolia')).toBe('evm');
  });
});

describe('getEscrowContractAddress', () => {
  /**
   * Test: EVM escrow address format
   * Why: EVM writes require a valid 20-byte hex address.
   */
  it('should return EVM escrow address for Base Sepolia', () => {
    const address = getEscrowContractAddress('base-sepolia');
    expect(address).toMatch(/^0x[a-fA-F0-9]{40}$/);
  });

  /**
   * Test: Missing EVM escrow address
   * Why: Misconfigured chains must fail fast with clear errors.
   */
  it('should throw if escrow address is missing', () => {
    expect(() => getEscrowContractAddress('movement')).toThrow(
      'Escrow contract address not configured for chain: movement'
    );
  });
});

describe('getSvmProgramId', () => {
  /**
   * Test: SVM program ID lookup
   * Why: SVM escrow instructions need a valid program ID.
   */
  it('should return SVM program ID for svm-devnet', () => {
    const programId = getSvmProgramId('svm-devnet');
    expect(programId).toMatch(/^[A-Za-z0-9]{32,44}$/);
  });

  /**
   * Test: Missing SVM program ID
   * Why: Misconfigured chains must fail fast with clear errors.
   */
  it('should throw if SVM program ID is missing', () => {
    expect(() => getSvmProgramId('movement')).toThrow(
      'SVM program ID not configured for chain: movement'
    );
  });
});
