let
  pkgs = import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/nixos-unstable.tar.gz";
    sha256 = "sha256:0mhqhq21y5vrr1f30qd2bvydv4bbbs5vyzhdp8xjf1vmqgyq4j49";
  }) {};
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    
    # Build dependencies
    pkg-config
    openssl
    openssl.dev
    
    # Development tools
    git
    curl
    wget
    
    # Optional: AWS CLI for S3 testing
    awscli2
    
    # Podman for MinIO testing (optional, preferred over Docker)
    # Note: podman-compose may need to be installed separately or use 'podman compose' subcommand
    podman
    
    # Platform-specific dependencies (Linux only)
  ] ++ pkgs.lib.optionals (pkgs.stdenv.isLinux) [
    linuxHeaders
  ];

  # Set environment variables
  shellHook = ''
    echo "Entering usync development environment"
    echo "Rust version: $(rustc --version)"
    echo "Cargo version: $(cargo --version)"
    echo ""
    echo "Available commands:"
    echo "  cargo build    - Build the project"
    echo "  cargo test     - Run tests"
    echo "  cargo fmt      - Format code"
    echo "  cargo clippy   - Run linter"
    echo ""
    echo "To exit, type: exit"
  '';
}

