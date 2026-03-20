import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['src/test/setup.ts'],
    env: {
      // Test-only placeholder addresses (valid format, not real deployments)
      NEXT_PUBLIC_MOVEMENT_TESTNET_INTENT_CONTRACT_ADDRESS: '0x' + 'aa'.repeat(32),
      NEXT_PUBLIC_BASE_TESTNET_ESCROW_CONTRACT_ADDRESS: '0x' + 'bb'.repeat(20),
      NEXT_PUBLIC_SOLANA_TESTNET_PROGRAM_ID: '11111111111111111111111111111111',
      NEXT_PUBLIC_SOLANA_TESTNET_GMP_ENDPOINT_ID: '22222222222222222222222222222222',
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
