<!--
SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>

SPDX-License-Identifier: AGPL-3.0-only
-->

# GitHub Copilot Instructions for top200-rs

**ALWAYS** follow these instructions first and only fallback to additional search and context gathering if the information in these instructions is incomplete or found to be in error.

## Project Overview

Top200-rs is a Rust CLI application that tracks and analyzes market capitalization data for top companies. It fetches data from financial APIs, stores it in SQLite, and provides various commands for analysis and export with visualization capabilities.

## Environment Setup and Dependencies

### System Dependencies
Install these system packages BEFORE attempting to build:
```bash
sudo apt update && sudo apt install -y pkg-config libfontconfig1-dev libssl-dev sqlite3
```

### Environment Variables
Create a `.env` file in the project root:
```bash
cp .env.example .env
```

Required environment variables:
- `DATABASE_URL=sqlite:data.db` (included in .env.example)
- `FINANCIALMODELINGPREP_API_KEY=your_api_key_here` (for API functionality)

### Rust Dependencies
Install additional Cargo tools needed for development:
```bash
cargo install sqlx-cli cargo-deny
pip install reuse  # For license compliance checking
```

## Building and Testing

### Critical Build Requirements
- **NEVER CANCEL BUILDS OR LONG-RUNNING COMMANDS**
- Set timeouts to 90+ minutes for clean builds
- Clean debug build: 45+ seconds, release build: 130+ seconds

### Database Setup
BEFORE building, prepare the SQLx query cache:
```bash
export DATABASE_URL=sqlite:data.db
cargo sqlx prepare
```

### Build Commands
```bash
# Build project (NEVER CANCEL - takes 45+ seconds clean)
DATABASE_URL=sqlite:data.db cargo build

# Release build (NEVER CANCEL - takes 130+ seconds clean)  
DATABASE_URL=sqlite:data.db cargo build --release

# Run tests (fast - ~0.1 seconds)
DATABASE_URL=sqlite:data.db cargo test
```

## Running the Application

### Basic Commands
```bash
# Show all available commands
DATABASE_URL=sqlite:data.db cargo run -- --help

# List currencies (works without API keys)
DATABASE_URL=sqlite:data.db cargo run -- list-currencies

# Export market caps (requires API keys)
DATABASE_URL=sqlite:data.db cargo run -- export-combined
```

### Key Subcommands
- `export-combined` - Export combined market caps to CSV
- `fetch-specific-date-market-caps DATE` - Fetch market caps for specific date
- `compare-market-caps --from DATE --to DATE` - Compare between dates
- `generate-charts --from DATE --to DATE` - Generate SVG visualizations
- `list-currencies` - List all supported currencies

## Code Quality and Validation

### Always Run Before Committing
```bash
# Format code (REQUIRED - CI will fail without)
cargo fmt --all

# Check linting (takes ~5 seconds)
cargo clippy

# Check license compliance (takes ~1 second)
reuse lint

# Dependency checking (may fail due to deprecated license identifiers)
cargo deny check  # Known to have license ID issues in deny.toml
```

### Test Validation
```bash
# Run all tests (fast - completes in ~0.1 seconds)
DATABASE_URL=sqlite:data.db cargo test

# Test specific functionality
DATABASE_URL=sqlite:data.db cargo test -- test_details_serialization
```

## Manual Validation Scenarios

After making changes, **ALWAYS** test these complete scenarios:

### Basic Functionality Test
```bash
# 1. Build succeeds
DATABASE_URL=sqlite:data.db cargo build

# 2. Help command works
DATABASE_URL=sqlite:data.db cargo run -- --help

# 3. List currencies (validates database connectivity)
DATABASE_URL=sqlite:data.db cargo run -- list-currencies
```

### Data Processing Workflow (if API keys available)
```bash
# Complete workflow test
DATABASE_URL=sqlite:data.db cargo run -- fetch-specific-date-market-caps 2025-01-01
DATABASE_URL=sqlite:data.db cargo run -- compare-market-caps --from 2024-12-31 --to 2025-01-01
```

## Important File Locations

### Core Source Files
- `src/main.rs` - CLI interface and command definitions
- `src/marketcaps.rs` - Core market cap data functionality
- `src/compare_marketcaps.rs` - Market cap comparison and analytics
- `src/visualizations.rs` - SVG chart generation
- `src/models.rs` - Data structures and serialization
- `src/api.rs` - External API client with rate limiting

### Configuration
- `config.toml` - Ticker symbols configuration
- `Cargo.toml` - Rust dependencies and project metadata
- `deny.toml` - Dependency licensing rules (has known deprecated license ID issues)
- `.rustfmt.toml` - Code formatting rules

### Database
- `migrations/` - SQLite database schema migrations
- `data.db` - SQLite database file (created automatically)
- `.sqlx/` - SQLx offline query cache (commit this)

## Known Issues and Workarounds

### Build Issues
- **Fontconfig error**: Install `libfontconfig1-dev` system package
- **SQLx query errors**: Set `DATABASE_URL` and run `cargo sqlx prepare`
- **OpenSSL errors**: Install `libssl-dev` system package

### Dependency Issues
- `cargo deny check` fails due to deprecated license identifiers in `deny.toml`
- Some files lack REUSE license headers (expected - not blocking)

### API Limitations
- FMP API: 300 requests per minute (enforced via semaphore)
- Requires valid API keys for full functionality
- Rate limiting and retry logic built-in

## Development Workflow

### For New Features
1. **ALWAYS** build first to ensure clean starting state
2. Run tests to verify existing functionality
3. Make changes incrementally
4. Test after each significant change
5. Run `cargo fmt --all` before committing
6. Run `cargo clippy` to check for issues
7. Validate with manual test scenarios

### For Bug Fixes
1. Write a test that reproduces the issue first
2. Verify the test fails
3. Make minimal changes to fix
4. Verify the test passes
5. Run full test suite to ensure no regressions

## Alternative Environments

### With Nix (Preferred for Full Development)
If Nix is available, use the provided development environment:
```bash
nix develop --command cargo build
nix develop --command cargo test
```

### Without Nix (Standard Cargo)
Follow the instructions above - the project builds fine with standard Cargo tools after installing system dependencies.

## CI/CD Integration

The project includes GitHub Actions workflows that:
- Run tests with coverage reporting
- Check code formatting with `cargo fmt`
- Run linting with `cargo clippy`
- Validate license compliance with REUSE
- Build and validate the Nix flake

**CRITICAL**: Format your code with `cargo fmt --all` before pushing or CI will fail.