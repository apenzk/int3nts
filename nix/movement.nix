{ stdenv, fetchurl, lib, gnutar, gzip, autoPatchelfHook, openssl, systemd }:

let
  # Movement CLI for testnet (Move 2 support)
  # Reference: https://docs.movementnetwork.xyz/devs/movementcli
  
  platform =
    if stdenv.isDarwin && stdenv.isAarch64 then "macos-arm64"
    else if stdenv.isDarwin then "macos-x86_64"
    else if stdenv.isLinux then "linux-x86_64"
    else throw "Unsupported platform ${stdenv.system}";

  # Movement CLI L1 release URLs (renamed from movement-move2-testnet-* to movement-cli-l1-*)
  urls = {
    "macos-arm64" = "https://github.com/movementlabsxyz/homebrew-movement-cli/releases/download/bypass-homebrew/movement-cli-l1-macos-arm64.tar.gz";
    "macos-x86_64" = "https://github.com/movementlabsxyz/homebrew-movement-cli/releases/download/bypass-homebrew/movement-cli-l1-macos-x86_64.tar.gz";
    "linux-x86_64" = "https://github.com/movementlabsxyz/homebrew-movement-cli/releases/download/bypass-homebrew/movement-cli-l1-linux-x86_64.tar.gz";
  };

  # SHA256 hashes for each platform
  hashes = {
    "macos-arm64" = "sha256-pWG8papab8XMIK7ylEHYQ702htm7/9VZwssZ11WVahI=";
    "macos-x86_64" = "sha256-dsywey53EGwRBUhyC6suD7ntW340jihyMa8upQOI+wI=";
    "linux-x86_64" = "sha256-8r3z+oLmtJyDyRvglnZAKC4XWeZrI3RAcJqHean5wbI=";
  };

in stdenv.mkDerivation rec {
  pname = "movement-cli";
  version = "l1";

  src = fetchurl {
    url = urls.${platform};
    sha256 = hashes.${platform};
  };

  nativeBuildInputs = [ gnutar gzip ] ++ lib.optionals stdenv.isLinux [ autoPatchelfHook ];
  buildInputs = [
    stdenv.cc.cc.lib  # libstdc++
    openssl           # libssl.so.3, libcrypto.so.3
  ] ++ lib.optionals stdenv.isLinux [
    systemd           # libudev.so.1
  ];

  unpackPhase = ''
    tar -xzf $src
  '';

  installPhase = ''
    mkdir -p $out/bin
    cp movement $out/bin/movement
    chmod +x $out/bin/movement
  '';

  meta = with lib; {
    description = "Movement CLI for Move 2 testnet";
    homepage = "https://docs.movementnetwork.xyz/devs/movementcli";
    platforms = [ "x86_64-darwin" "aarch64-darwin" "x86_64-linux" ];
  };
}

