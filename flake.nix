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
        # Include Cargo sources plus migrations and .sqlx metadata for SQLx offline
        src =
          let
            filtered = pkgs.lib.cleanSourceWith {
              src = ./.;
              filter = path: type:
                let
                  rel = pkgs.lib.removePrefix ((toString ./. ) + "/") (toString path);
                in pkgs.lib.any (p: pkgs.lib.hasPrefix p rel) [
                  "src"
                  "migrations"
                  ".sqlx"
                  "Cargo.toml"
                  "Cargo.lock"
                  "build.rs"
                ];
            };
          in craneLib.cleanCargoSource filtered;
        # as before
        nativeBuildInputs = with pkgs; [ rustToolchain pkg-config ];
        buildInputs = with pkgs; [ openssl sqlite fontconfig ];
        # because we'll use it for both `cargoArtifacts` and `bin`
        commonArgs = {
          inherit src buildInputs nativeBuildInputs;
          # Set DATABASE_URL for SQLx compile-time checking
          DATABASE_URL = "sqlite://.sqlx/build.db?mode=rwc";
          SQLX_DISABLE_DEFAULT_DOTENV = "1";
          SQLX_OFFLINE = "1";
          RUSTFLAGS = "--cfg sqlx_macros_offline";
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        # remember, `set1 // set2` does a shallow merge:
        buildNativeInputs = nativeBuildInputs ++ [ pkgs.sqlx-cli ];
        bin = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          nativeBuildInputs = buildNativeInputs;
          preBuild = ''
            set -euo pipefail
            echo "Preparing SQLx offline data..."
            export SQLX_DISABLE_DEFAULT_DOTENV=1
            export SQLX_OFFLINE=0
            mkdir -p .sqlx
            DBURL="sqlite://$PWD/.sqlx/build.db?mode=rwc"
            export DATABASE_URL="$DBURL"
            ${pkgs.sqlx-cli}/bin/sqlx database create --database-url "$DBURL" || true
            ${pkgs.sqlx-cli}/bin/sqlx migrate run --database-url "$DBURL"
            cargo sqlx prepare -D "$DBURL" -- --bin top200-rs
            export SQLX_OFFLINE=1
          '';
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

        apps =
          let
            top200Exe = "${bin}/bin/top200-rs";
            commonRuntimeInputs = with pkgs; [ coreutils bash sqlite openssl cacert git pkg-config sqlx-cli ];
            cargoBin = "${rustToolchain}/bin/cargo";

            runSpecificDate = pkgs.writeShellApplication {
              name = "specific-date";
              runtimeInputs = commonRuntimeInputs ++ [ rustToolchain ];
              text = ''
                set -euo pipefail

                DATE="''${1:-}"
                if [ -z "''${DATE}" ]; then
                  echo "Usage: specific-date YYYY-MM-DD" >&2
                  exit 1
                fi

                export DATABASE_URL="''${DATABASE_URL:-sqlite:data.db}"
                export SSL_CERT_FILE="''${SSL_CERT_FILE:-${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt}"
                export NIX_SSL_CERT_FILE="''${SSL_CERT_FILE}"
                export SQLX_DISABLE_DEFAULT_DOTENV=1

                if [ -z "''${FINANCIALMODELINGPREP_API_KEY:-}" ]; then
                  echo "Error: FINANCIALMODELINGPREP_API_KEY is not set" >&2
                  exit 1
                fi

                # Prepare SQLx offline data and build
                export SQLX_DISABLE_DEFAULT_DOTENV=1
                export SQLX_OFFLINE=1
                HAD_ENV=0
                if [ -f .env ]; then
                  HAD_ENV=1
                  mv .env .env.backup.sqlx
                  trap 'if [ "$HAD_ENV" = "1" ]; then mv -f .env.backup.sqlx .env 2>/dev/null || true; fi' EXIT
                fi
                if [ ! -f sqlx-data.json ]; then
                  SQLX_OFFLINE=0 ${cargoBin} sqlx prepare -- --bin top200-rs
                fi
                ${cargoBin} build --release
                exec ./target/release/top200-rs fetch-specific-date-market-caps "''${DATE}"
              '';
            };

            runCompare = pkgs.writeShellApplication {
              name = "compare";
              runtimeInputs = commonRuntimeInputs ++ [ rustToolchain ];
              text = ''
                set -euo pipefail

                FROM="''${1:-}"
                TO="''${2:-}"
                if [ -z "''${FROM}" ] || [ -z "''${TO}" ]; then
                  echo "Usage: compare FROM_DATE TO_DATE (YYYY-MM-DD YYYY-MM-DD)" >&2
                  exit 1
                fi

                export DATABASE_URL="''${DATABASE_URL:-sqlite:data.db}"
                export SSL_CERT_FILE="''${SSL_CERT_FILE:-${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt}"
                export NIX_SSL_CERT_FILE="''${SSL_CERT_FILE}"
                export SQLX_DISABLE_DEFAULT_DOTENV=1

                export SQLX_DISABLE_DEFAULT_DOTENV=1
                export SQLX_OFFLINE=1
                HAD_ENV=0
                if [ -f .env ]; then
                  HAD_ENV=1
                  mv .env .env.backup.sqlx
                  trap 'if [ "$HAD_ENV" = "1" ]; then mv -f .env.backup.sqlx .env 2>/dev/null || true; fi' EXIT
                fi
                if [ ! -f sqlx-data.json ]; then
                  SQLX_OFFLINE=0 ${cargoBin} sqlx prepare -- --bin top200-rs
                fi
                ${cargoBin} build --release
                exec ./target/release/top200-rs compare-market-caps --from "''${FROM}" --to "''${TO}"
              '';
            };

            runVisualize = pkgs.writeShellApplication {
              name = "generate-charts";
              runtimeInputs = commonRuntimeInputs ++ [ rustToolchain ];
              text = ''
                set -euo pipefail

                FROM="''${1:-}"
                TO="''${2:-}"
                if [ -z "''${FROM}" ] || [ -z "''${TO}" ]; then
                  echo "Usage: generate-charts FROM_DATE TO_DATE (YYYY-MM-DD YYYY-MM-DD)" >&2
                  exit 1
                fi

                export DATABASE_URL="''${DATABASE_URL:-sqlite:data.db}"
                export SSL_CERT_FILE="''${SSL_CERT_FILE:-${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt}"
                export NIX_SSL_CERT_FILE="''${SSL_CERT_FILE}"
                export SQLX_DISABLE_DEFAULT_DOTENV=1

                export SQLX_DISABLE_DEFAULT_DOTENV=1
                export SQLX_OFFLINE=1
                HAD_ENV=0
                if [ -f .env ]; then
                  HAD_ENV=1
                  mv .env .env.backup.sqlx
                  trap 'if [ "$HAD_ENV" = "1" ]; then mv -f .env.backup.sqlx .env 2>/dev/null || true; fi' EXIT
                fi
                if [ ! -f sqlx-data.json ]; then
                  SQLX_OFFLINE=0 ${cargoBin} sqlx prepare -- --bin top200-rs
                fi
                ${cargoBin} build --release
                exec ./target/release/top200-rs generate-charts --from "''${FROM}" --to "''${TO}"
              '';
            };

            runPipeline = pkgs.writeShellApplication {
              name = "pipeline-today-vs-7days";
              runtimeInputs = commonRuntimeInputs ++ [ rustToolchain ];
              text = ''
                set -euo pipefail

                # Compute dates in UTC using GNU coreutils date
                TODAY="$(${pkgs.coreutils}/bin/date -u +%Y-%m-%d)"
                SEVEN_AGO="$(${pkgs.coreutils}/bin/date -u -d '7 days ago' +%Y-%m-%d)"

                echo "Running pipeline for ''${SEVEN_AGO} -> ''${TODAY}"

                export DATABASE_URL="''${DATABASE_URL:-sqlite:data.db}"
                export SSL_CERT_FILE="''${SSL_CERT_FILE:-${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt}"
                export NIX_SSL_CERT_FILE="''${SSL_CERT_FILE}"
                export SQLX_DISABLE_DEFAULT_DOTENV=1

                if [ -z "''${FINANCIALMODELINGPREP_API_KEY:-}" ]; then
                  echo "Error: FINANCIALMODELINGPREP_API_KEY is not set" >&2
                  exit 1
                fi

                # Prepare SQLx offline data and build once
                export SQLX_DISABLE_DEFAULT_DOTENV=1
                export SQLX_OFFLINE=1
                HAD_ENV=0
                if [ -f .env ]; then
                  HAD_ENV=1
                  mv .env .env.backup.sqlx
                  trap 'if [ "$HAD_ENV" = "1" ]; then mv -f .env.backup.sqlx .env 2>/dev/null || true; fi' EXIT
                fi
                if [ ! -f sqlx-data.json ]; then
                  SQLX_OFFLINE=0 ${cargoBin} sqlx prepare -- --bin top200-rs
                fi
                ${cargoBin} build --release

                # Fetch market caps for both dates
                ./target/release/top200-rs fetch-specific-date-market-caps "''${SEVEN_AGO}"
                ./target/release/top200-rs fetch-specific-date-market-caps "''${TODAY}"

                # Compare and generate charts
                ./target/release/top200-rs compare-market-caps --from "''${SEVEN_AGO}" --to "''${TODAY}"
                ./target/release/top200-rs generate-charts --from "''${SEVEN_AGO}" --to "''${TODAY}"

                echo "✅ Pipeline completed for ''${SEVEN_AGO} -> ''${TODAY}"
              '';
            };

            runExportCombined = pkgs.writeShellApplication {
              name = "export-combined";
              runtimeInputs = commonRuntimeInputs ++ [ rustToolchain ];
              text = ''
                set -euo pipefail

                export DATABASE_URL="''${DATABASE_URL:-sqlite:data.db}"
                export SSL_CERT_FILE="''${SSL_CERT_FILE:-${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt}"
                export NIX_SSL_CERT_FILE="''${SSL_CERT_FILE}"

                if [ -z "''${FINANCIALMODELINGPREP_API_KEY:-}" ]; then
                  echo "Error: FINANCIALMODELINGPREP_API_KEY is not set" >&2
                  exit 1
                fi

                export SQLX_DISABLE_DEFAULT_DOTENV=1
                export SQLX_OFFLINE=1
                if [ ! -f sqlx-data.json ]; then
                  SQLX_OFFLINE=0 sqlx prepare -- --bin top200-rs
                fi
                ${cargoBin} build --release
                exec ./target/release/top200-rs export-combined
              '';
            };

            runBuild = pkgs.writeShellApplication {
              name = "build";
              runtimeInputs = commonRuntimeInputs ++ [ rustToolchain ];
              text = ''
                set -euo pipefail
                export DATABASE_URL="''${DATABASE_URL:-sqlite:data.db}"
                export SSL_CERT_FILE="''${SSL_CERT_FILE:-${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt}"
                export NIX_SSL_CERT_FILE="''${SSL_CERT_FILE}"
                export SQLX_DISABLE_DEFAULT_ENV_FILE=1
                export SQLX_DISABLE_DEFAULT_DOTENV=1
                export SQLX_OFFLINE=1
                HAD_ENV=0
                if [ -f .env ]; then
                  HAD_ENV=1
                  mv .env .env.backup.sqlx
                  trap 'if [ "$HAD_ENV" = "1" ]; then mv -f .env.backup.sqlx .env 2>/dev/null || true; fi' EXIT
                fi
                if [ ! -f sqlx-data.json ]; then
                  SQLX_OFFLINE=0 ${cargoBin} sqlx prepare -- --bin top200-rs
                fi
                exec ${cargoBin} build --release
              '';
            };
          in
          {
            default = {
              type = "app";
              program = top200Exe;
            };

            top200 = {
              type = "app";
              program = top200Exe;
            };

            "specific-date" = {
              type = "app";
              program = "${runSpecificDate}/bin/specific-date";
            };

            compare = {
              type = "app";
              program = "${runCompare}/bin/compare";
            };

            "generate-charts" = {
              type = "app";
              program = "${runVisualize}/bin/generate-charts";
            };

            "pipeline-today-vs-7days" = {
              type = "app";
              program = "${runPipeline}/bin/pipeline-today-vs-7days";
            };

            "export-combined" = {
              type = "app";
              program = "${runExportCombined}/bin/export-combined";
            };

            build = {
              type = "app";
              program = "${runBuild}/bin/build";
            };
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
            export SQLX_OFFLINE=1
            export SQLX_DISABLE_DEFAULT_DOTENV=1
            export RUSTFLAGS="--cfg sqlx_macros_offline ${RUSTFLAGS:-}"
            echo "🦀 Welcome to the top200-rs development environment!"
          '';
        };
      }
    );
}
