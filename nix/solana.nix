{ pkgs, ... }:

# Use solana-cli from nixpkgs (version 2.3.13)
# Anchor CLI is installed via cargo in the shellHook
pkgs.solana-cli
