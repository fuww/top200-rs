# SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
# SPDX-License-Identifier: AGPL-3.0-only

# Top200-rs Justfile - Common development tasks
# Run `just --list` to see all available commands

# Default recipe - show available commands
default:
    @just --list

# =============================================================================
# Build Commands
# =============================================================================

# Build the project in debug mode
build:
    nix develop --command cargo build

# Build the project in release mode
build-release:
    nix develop --command cargo build --release

# Check code without building
check:
    nix develop --command cargo check

# Clean build artifacts
clean:
    nix develop --command cargo clean

# =============================================================================
# Testing
# =============================================================================

# Run all tests
test:
    nix develop --command cargo test

# Run tests with output
test-verbose:
    nix develop --command cargo test -- --nocapture

# Run a specific test by name
test-one name:
    nix develop --command cargo test {{name}}

# Run tests with coverage report
test-coverage:
    nix develop --command cargo tarpaulin --out lcov --output-dir coverage

# =============================================================================
# Linting & Formatting
# =============================================================================

# Format all code
fmt:
    nix develop --command cargo fmt --all

# Check formatting without changes
fmt-check:
    nix develop --command cargo fmt --all -- --check

# Run clippy linter
lint:
    nix develop --command cargo clippy

# Run clippy with all warnings as errors
lint-strict:
    nix develop --command cargo clippy -- -D warnings

# Check license compliance
license-check:
    reuse lint

# Run cargo-deny for dependency checks
deny:
    nix develop --command cargo deny check

# Run all quality checks (format, lint, deny)
quality: fmt lint deny

# =============================================================================
# Database Operations
# =============================================================================

# Open SQLite database shell
db:
    sqlite3 data.db

# Show database schema
db-schema:
    sqlite3 data.db ".schema"

# Show all tables
db-tables:
    sqlite3 data.db ".tables"

# Show database statistics
db-stats:
    @echo "=== Database Statistics ==="
    @echo "Tables and row counts:"
    @sqlite3 data.db "SELECT 'market_caps', COUNT(*) FROM market_caps;"
    @sqlite3 data.db "SELECT 'forex_rates', COUNT(*) FROM forex_rates;"
    @sqlite3 data.db "SELECT 'currencies', COUNT(*) FROM currencies;"
    @sqlite3 data.db "SELECT 'ticker_details', COUNT(*) FROM ticker_details;"
    @sqlite3 data.db "SELECT 'symbol_changes', COUNT(*) FROM symbol_changes;"

# =============================================================================
# Application Commands
# =============================================================================

# Run the application (default: market caps)
run *args:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- {{args}}

# Show help
help:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- --help

# Fetch current exchange rates
rates:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- ExportRates

# Fetch historical exchange rates (example: just rates-historical 2024-01-01 2024-12-31)
rates-historical from to:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-historical-exchange-rates --from {{from}} --to {{to}}

# Backfill exchange rates for last year
rates-backfill-year:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-historical-exchange-rates --from $(date -d "1 year ago" +%Y-%m-%d) --to $(date +%Y-%m-%d)

# Fetch market caps for today
marketcaps:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- ExportCombined

# Fetch market caps for a specific date (example: just marketcaps-date 2025-01-15)
marketcaps-date date:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-specific-date-market-caps {{date}}

# Fetch historical market caps for a year range
marketcaps-historical start end:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- FetchHistoricalMarketCaps {{start}} {{end}}

# Compare market caps between two dates
compare from to:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- compare-market-caps --from {{from}} --to {{to}}

# Generate visualization charts
charts from to:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- generate-charts --from {{from}} --to {{to}}

# Full comparison workflow: fetch both dates, compare, and generate charts
compare-full from to:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-specific-date-market-caps {{from}}
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-specific-date-market-caps {{to}}
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- compare-market-caps --from {{from}} --to {{to}}
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- generate-charts --from {{from}} --to {{to}}

# Year-to-date comparison
ytd:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-specific-date-market-caps $(date -d "last year" +%Y)-12-31
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- fetch-specific-date-market-caps $(date +%Y-%m-%d)
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- compare-market-caps --from $(date -d "last year" +%Y)-12-31 --to $(date +%Y-%m-%d)

# List all currencies
currencies:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- ListCurrencies

# Check for symbol changes
symbol-check:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- check-symbol-changes

# Preview symbol changes (dry run)
symbol-preview:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- apply-symbol-changes --dry-run

# Apply symbol changes automatically
symbol-apply:
    DATABASE_URL=sqlite:data.db nix develop --command cargo run -- apply-symbol-changes --auto-apply

# =============================================================================
# Development Workflow
# =============================================================================

# Enter nix development shell
dev:
    nix develop

# Watch for changes and rebuild
watch:
    nix develop --command cargo watch -x check

# Full CI check (what CI runs)
ci: fmt-check lint test deny

# Pre-commit hook simulation
pre-commit: fmt lint test

# =============================================================================
# Documentation
# =============================================================================

# Generate and open documentation
docs:
    nix develop --command cargo doc --open

# Generate documentation without opening
docs-build:
    nix develop --command cargo doc

# =============================================================================
# Utility
# =============================================================================

# Update nix flake
update-flake:
    nix flake update

# Show outdated dependencies
outdated:
    nix develop --command cargo outdated

# Count lines of code
loc:
    @echo "=== Lines of Code ==="
    @find src -name "*.rs" | xargs wc -l | tail -1
    @echo ""
    @echo "=== By file ==="
    @find src -name "*.rs" | xargs wc -l | sort -n

# Show project structure
tree:
    @tree -I 'target|output|.git|coverage' --dirsfirst
