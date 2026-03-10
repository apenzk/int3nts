//! Shared helpers for EVM Hardhat scripts
//!
//! Common utilities extracted to eliminate duplication across scripts.

const hre = require("hardhat");

/// Validates that all required environment variables are set.
///
/// Throws an error listing the missing variables if any are unset.
function requireEnvVars(names) {
  const values = {};
  const missing = [];
  for (const name of names) {
    const val = process.env[name];
    if (!val) {
      missing.push(name);
    } else {
      values[name] = val;
    }
  }
  if (missing.length > 0) {
    const error = new Error(
      `Missing required environment variables: ${missing.join(", ")}`
    );
    console.error("Error:", error.message);
    if (require.main === module) {
      process.exit(1);
    }
    throw error;
  }
  return values;
}

/// Normalizes a hex intent ID to bytes32 format (0x + 64 hex chars).
///
/// Ensures 0x prefix and left-pads with zeros to 32 bytes.
function toBytes32(hexStr) {
  let s = hexStr;
  if (!s.startsWith("0x")) {
    s = "0x" + s;
  }
  return "0x" + s.slice(2).padStart(64, "0");
}

/// Extracts a 20-byte EVM address from a potentially 32-byte padded format.
///
/// 32-byte format: 0x000000000000000000000000<20-byte-address>
/// 20-byte format: 0x<20-byte-address> (returned as-is)
function toEvmAddress(addr) {
  if (addr.length === 66) {
    return "0x" + addr.slice(-40);
  }
  return addr;
}

/// Returns the solver signer for the current Hardhat network.
///
/// - Hardhat in-memory network (unit tests): uses signers[2]
/// - External networks with SOLVER_EVM_PRIVATE_KEY: creates wallet from key
/// - External networks without key: errors
async function getSolverSigner() {
  if (hre.network.name === "hardhat") {
    const signers = await hre.ethers.getSigners();
    if (signers.length < 3) {
      throw new Error(`Expected at least 3 signers, got ${signers.length}`);
    }
    return signers[2];
  }

  if (process.env.SOLVER_EVM_PRIVATE_KEY) {
    const { ethers } = require("ethers");
    const rpcUrl = hre.network.config.url || "http://127.0.0.1:8545";
    const provider = new ethers.JsonRpcProvider(rpcUrl);
    return new ethers.Wallet(process.env.SOLVER_EVM_PRIVATE_KEY, provider);
  }

  throw new Error("SOLVER_EVM_PRIVATE_KEY is required for non-local networks");
}

/// Standard main() runner with error handling and process.exit.
///
/// Wraps a main function with the standard pattern used across all scripts.
/// The caller must pass its own `module` so the entry-point check works correctly.
///
/// Usage: `runMain(main, module);`
function runMain(mainFn, callerModule) {
  if (require.main === callerModule) {
    mainFn()
      .then(() => process.exit(0))
      .catch((error) => {
        console.error("Error:", error.message);
        process.exit(1);
      });
  }
}

module.exports = {
  requireEnvVars,
  toBytes32,
  toEvmAddress,
  getSolverSigner,
  runMain,
};
