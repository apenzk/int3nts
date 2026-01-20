{
  description = "Intent Framework dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        aptosCli = pkgs.callPackage ./aptos.nix {};
        movementCli = pkgs.callPackage ./movement.nix {};
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.rustfmt
            pkgs.clippy
            pkgs.jq
            pkgs.curl
            pkgs.bash
            pkgs.coreutils
            pkgs.openssl
            pkgs.pkg-config
            pkgs.nodejs
            pkgs.nodePackages.npm
            pkgs.git
            pkgs.libiconv  # Required for Rust on macOS
            aptosCli      # For local Docker e2e testing
            movementCli   # For testnet deployment
            # Solana CLI installed via official script in shellHook (needs writable dir for platform-tools)
            # SVM builds use scripts/build.sh which handles Solana's toolchain requirements
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.eudev  # libudev headers needed for node-usb (frontend tests)
          ];

          shellHook = ''
            # Solana/rustup tools path (added AFTER Nix tools, so Nix Rust takes precedence)
            # SVM build script explicitly uses rustup's cargo when needed
            export PATH="$PATH:$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin"
            
            # Install rustup if not already installed (needed for Solana's +toolchain syntax)
            if ! command -v rustup > /dev/null 2>&1; then
              echo "[nix] Installing rustup (needed for Solana builds)..."
              curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable 2>/dev/null || true
            fi
            
            # Install Solana CLI if not already installed (official installer, writable location)
            # Required for SVM tests (cargo build-sbf)
            # Set SKIP_SOLANA=1 to skip installation (e.g., for EVM/MVM CI jobs)
            if [ -z "''${SKIP_SOLANA:-}" ]; then
              if ! command -v solana > /dev/null 2>&1; then
                echo "[nix] Installing Solana CLI..."
                sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)" 2>/dev/null || true
              fi
            else
              echo "[nix] Skipping Solana CLI install (SKIP_SOLANA is set)"
            fi
            
            echo "[nix] Dev shell ready: rustc $(rustc --version 2>/dev/null | awk '{print $2}' || echo 'not installed') | cargo $(cargo --version 2>/dev/null | awk '{print $2}' || echo 'not installed') | aptos $(aptos --version 2>/dev/null || echo 'unknown') | movement $(movement --version 2>/dev/null || echo 'unknown') | solana $(solana --version 2>/dev/null | head -1 | awk '{print $2}' || echo 'not installed') | node $(node --version 2>/dev/null || echo 'unknown')"
            
            export OPENSSL_DIR=${pkgs.openssl.dev}
            export OPENSSL_LIB_DIR=${pkgs.openssl.out}/lib
            export OPENSSL_INCLUDE_DIR=${pkgs.openssl.dev}/include
          '';
        };
      }
    );
}
