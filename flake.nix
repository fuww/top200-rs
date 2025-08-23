# SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
#
# SPDX-License-Identifier: AGPL-3.0-only

{
  description = "Development environment for top200-rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        # Latest stable Rust
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        # this is how we can tell crane to use our toolchain!
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        # cf. https://crane.dev/API.html#libcleancargosource
        src = craneLib.cleanCargoSource ./.;
        # as before
        nativeBuildInputs = with pkgs; [ rustToolchain pkg-config ];
        buildInputs = with pkgs; [ openssl sqlite fontconfig ];
        # because we'll use it for both `cargoArtifacts` and `bin`
        commonArgs = {
          inherit src buildInputs nativeBuildInputs;
          # Set DATABASE_URL for SQLx compile-time checking
          DATABASE_URL = "sqlite:data.db";
          # Create temporary database for SQLx compile-time verification
          preBuild = ''
            echo "Creating temporary database for SQLx..."
            ${pkgs.sqlite}/bin/sqlite3 data.db < migrations/20250123_init.sql
            ${pkgs.sqlite}/bin/sqlite3 data.db < migrations/20250124123300_create_ticker_details.sql
            ${pkgs.sqlite}/bin/sqlite3 data.db < migrations/20250817140000_create_symbol_changes.sql
            echo "Database created successfully"
          '';
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        # remember, `set1 // set2` does a shallow merge:
        bin = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
         packages =
            {
              # that way we can build `bin` specifically,
              # but it's also the default.
              inherit bin;
              default = bin;
            };

        devShells.default = pkgs.mkShell {
          # instead of passing `buildInputs` / `nativeBuildInputs`,
            # we refer to an existing derivation here
            inputsFrom = [ bin ];
            buildInputs = with pkgs; [ reuse clippy-sarif sarif-fmt sqlite sqlx-cli cargo-deny ];
          # buildInputs = with pkgs; [
          #   # Rust toolchain
          #   rustToolchain

          #   # Additional dependencies
          #   pkg-config
          #   openssl
          #   trunk

          #   # SQLite for database
          #   sqlite
          # ];

          # Environment variables
          shellHook = ''
            export DATABASE_URL=sqlite:data.db
            echo "🦀 Welcome to the top200-rs development environment!"
          '';
        };
      }
    );
}
