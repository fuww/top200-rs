# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is a Rust application that tracks and analyzes market capitalization data for top companies (Top200-rs). It fetches data from financial APIs, stores it in SQLite, and provides various commands for analysis and export.

## Building and Running

This project uses a Nix development environment.
Prefix commands with `nix develop --command` to run them in the Nix environment. In the docs we put regular commands without the prefix to be concise.

### Development Environment Setup

```bash
# Clone and enter the repository
git clone https://github.com/javdl/top200-rs.git
cd top200-rs

# Set up environment using Nix
nix develop

# Or run directly with Nix
nix develop --command cargo run
```

### Environment Variables

Create a `.env` file in the project root with:

```env
FMP_API_KEY=your_api_key_here
FINANCIALMODELINGPREP_API_KEY=your_api_key_here
DATABASE_URL=sqlite:data.db  # Optional, defaults to sqlite:data.db
```

### Build Commands

```bash
# Build the project
cargo build

# Build for release
cargo build --release
```

### Run Commands

```bash
# Run without arguments (defaults to marketcaps subcommand)
cargo run

# Run with help to see all commands
cargo run -- --help

# Run a specific subcommand
cargo run -- ExportCombined
cargo run -- ListCurrencies
cargo run -- FetchHistoricalMarketCaps 2022 2025
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_details_serialization

# Run tests with coverage
cargo tarpaulin --out lcov --output-dir coverage
```

## Linting and Formatting

```bash
# Format code
cargo fmt --all

# Run clippy linter
cargo clippy

# Check license compliance
reuse lint
```

## Database Operations

The application uses SQLite with SQLx for database operations. Migrations are located in the `migrations/` directory.

```bash
# Inspect database (using sqlite3 CLI)
sqlite3 data.db

# Run a specific SQL query from tests
sqlite3 data.db < tests/market_caps_totals_per_year.sql
```

## Code Architecture

### Core Components

1. **API Clients**: Abstraction layer for external APIs
   - Financial Modeling Prep (FMP) API client in `src/api.rs`
   - Rate limiting with tokio semaphore (300 req/min for FMP)

2. **Data Models**: Defined in `src/models.rs`
   - Company details
   - Financial data
   - Exchange rates

3. **Database Layer**: Handles SQLite operations and migrations
   - Connection pooling with SQLx
   - Automatic migrations on startup
   - Tables: `currencies`, `forex_rates`, `market_caps`, `ticker_details`

4. **Commands**: CLI interface using clap for parsing arguments

### Data Flow

1. Fetch exchange rates for currency conversion
2. Retrieve market cap data from various sources
3. Store in SQLite database
4. Generate reports (CSV exports, charts)

### Key Modules

- `marketcaps.rs`: Core functionality for market cap data
- `compare_marketcaps.rs`: Compare market caps between dates with analytics
- `exchange_rates.rs`: Currency exchange rate handling
- `details_*.rs`: Company details from different sources
- `historical_marketcaps.rs`: Historical data retrieval
- `monthly_historical_marketcaps.rs`: Monthly historical data
- `specific_date_marketcaps.rs`: Fetch market caps for specific dates
- `ticker_details.rs`: Company details management
- `utils.rs`: Common utilities and helpers
- `visualizations.rs`: Generate beautiful SVG charts from comparison data
- `advanced_comparisons.rs`: Multi-date trends, YoY/QoQ, rolling periods, benchmarks, peer groups

## Common Tasks

### Adding New Tickers

Edit the `config.toml` file to add new tickers to either the `us_tickers` or `non_us_tickers` arrays.

### Updating Exchange Rates

```bash
# Fetch current exchange rates
cargo run -- ExportRates

# Backfill historical exchange rates for a date range
cargo run -- fetch-historical-exchange-rates --from 2024-01-01 --to 2024-12-31

# This command will:
# - Fetch daily exchange rates for common currency pairs (EUR, GBP, JPY, CHF, etc.)
# - Store rates in the database with their respective dates
# - Enable accurate historical market cap comparisons with correct FX rates
```

### Generating Combined Market Cap Reports

```bash
cargo run -- ExportCombined
```

### Working with Historical Data

```bash
# Fetch historical market caps for a range of years
cargo run -- FetchHistoricalMarketCaps 2023 2025

# Fetch monthly historical market caps
cargo run -- FetchMonthlyHistoricalMarketCaps 2023 2025
```

### Fetching Market Caps for a Specific Date

```bash
# Fetch market caps for a specific date (format: YYYY-MM-DD)
cargo run -- fetch-specific-date-market-caps 2025-08-01

# This command will:
# - Fetch market cap data for all configured tickers for the specified date
# - Retrieve exchange rates from the database
# - Export the data to a CSV file in the output/ directory
# - File format: marketcaps_YYYY-MM-DD_YYYYMMDD_HHMMSS.csv
```

### Comparing Market Caps Between Dates

```bash
# Compare market caps between two dates (format: YYYY-MM-DD)
cargo run -- compare-market-caps --from 2025-07-01 --to 2025-08-01

# This command will:
# - Read the market cap CSV files for both dates
# - Calculate absolute and percentage changes
# - Compute ranking changes and market share shifts
# - Export a detailed comparison CSV
# - Generate a summary report in Markdown format
# Output files:
# - comparison_YYYY-MM-DD_to_YYYY-MM-DD_YYYYMMDD_HHMMSS.csv
# - comparison_YYYY-MM-DD_to_YYYY-MM-DD_summary_YYYYMMDD_HHMMSS.md

# Year-to-date comparison (fetch end of last year and compare with today)
cargo run -- fetch-specific-date-market-caps 2024-12-31 && \
cargo run -- fetch-specific-date-market-caps $(date +%Y-%m-%d) && \
cargo run -- compare-market-caps --from 2024-12-31 --to $(date +%Y-%m-%d)
```

### Generating Visualization Charts

```bash
# Generate beautiful SVG charts from comparison data
cargo run -- generate-charts --from 2025-07-01 --to 2025-08-01

# This command will:
# - Find the comparison CSV file for the specified dates
# - Generate 4 professional visualization charts:
#   1. Top Gainers and Losers bar chart (horizontal bars with gradient colors)
#   2. Market Cap Distribution donut chart (shows top 10 companies + others)
#   3. Rank Movements chart (shows biggest rank improvements and declines)
#   4. Market Summary Dashboard (comprehensive overview with metrics and pie chart)
# - Export all charts as SVG files to the output/ directory

# Complete workflow example:
cargo run -- fetch-specific-date-market-caps 2025-07-01 && \
cargo run -- fetch-specific-date-market-caps 2025-08-01 && \
cargo run -- compare-market-caps --from 2025-07-01 --to 2025-08-01 && \
cargo run -- generate-charts --from 2025-07-01 --to 2025-08-01
```

### Advanced Comparison Features

#### Multi-date Trend Analysis

Compare more than 2 dates to analyze trends over time:

```bash
# Analyze trends across multiple dates
cargo run -- trend-analysis --dates 2025-01-01,2025-04-01,2025-07-01,2025-10-01

# This command will:
# - Compare market caps across all specified dates
# - Calculate CAGR (Compound Annual Growth Rate)
# - Measure volatility and max drawdown
# - Identify best/worst performers and most volatile stocks
# Output files:
# - trend_analysis_YYYY-MM-DD_to_YYYY-MM-DD_YYYYMMDD_HHMMSS.csv
# - trend_analysis_YYYY-MM-DD_to_YYYY-MM-DD_summary_YYYYMMDD_HHMMSS.md
```

#### Year-over-Year (YoY) Comparison

Automatic year-over-year analysis:

```bash
# Compare current date with same date in previous years
cargo run -- compare-yoy --date 2025-06-15 --years 3

# This will compare: 2022-06-15, 2023-06-15, 2024-06-15, 2025-06-15
# Requires market cap data for all these dates
```

#### Quarter-over-Quarter (QoQ) Comparison

Quarterly trend analysis:

```bash
# Compare quarterly data
cargo run -- compare-qoq --date 2025-06-30 --quarters 4

# This will compare quarter-end dates going back 4 quarters
# Quarter-end dates: Mar 31, Jun 30, Sep 30, Dec 31
```

#### Rolling Period Comparisons

Compare with rolling time windows:

```bash
# 30-day rolling comparison
cargo run -- compare-rolling --date 2025-06-15 --period 30d

# 90-day rolling comparison
cargo run -- compare-rolling --date 2025-06-15 --period 90d

# 1-year rolling comparison
cargo run -- compare-rolling --date 2025-06-15 --period 1y

# Custom period (e.g., 45 days)
cargo run -- compare-rolling --date 2025-06-15 --period 45d
```

#### Benchmark Comparison

Compare performance against market benchmarks:

```bash
# Compare against S&P 500 (uses total market cap as proxy)
cargo run -- compare-benchmark --from 2025-01-01 --to 2025-06-15 --benchmark sp500

# Compare against MSCI World
cargo run -- compare-benchmark --from 2025-01-01 --to 2025-06-15 --benchmark msci

# Output shows:
# - Relative performance vs benchmark
# - Outperformers and underperformers
# - Detailed comparison with benchmark returns
```

#### Peer Group Comparison

Compare predefined industry peer groups:

```bash
# Compare all peer groups
cargo run -- compare-peer-groups --from 2025-01-01 --to 2025-06-15

# Compare specific groups
cargo run -- compare-peer-groups --from 2025-01-01 --to 2025-06-15 --groups luxury,sportswear

# Available peer groups:
# - Luxury (LVMH, Hermès, Kering, Dior, Richemont, etc.)
# - Sportswear (Nike, Adidas, Puma, Lululemon, etc.)
# - Fast Fashion (Inditex, H&M, Fast Retailing, Gap, etc.)
# - Department Stores (Macy's, Nordstrom, Kohl's, etc.)
# - Value Retail (TJX, Ross Stores, Burlington, etc.)
# - Footwear (Nike, Birkenstock, Crocs, Deckers, etc.)
# - E-commerce (Zalando, Vipshop, Revolve, etc.)
# - Asian Fashion (Fast Retailing, Li Ning, Bosideng, etc.)
```

#### Utility Commands

```bash
# List available dates for comparison (from output/ directory)
cargo run -- list-available-dates

# List predefined peer groups with their tickers
cargo run -- list-peer-groups
```

### Tracking Stock Symbol Changes

The application can track and apply stock ticker symbol changes (due to mergers, acquisitions, rebranding, etc.):

```bash
# Check for symbol changes that affect our tickers
cargo run -- check-symbol-changes

# Apply symbol changes with dry run (preview changes)
cargo run -- apply-symbol-changes --dry-run

# Automatically apply all non-conflicting changes
cargo run -- apply-symbol-changes --auto-apply

# Specify a custom config file
cargo run -- check-symbol-changes --config custom-config.toml
```

Symbol changes are fetched from the Financial Modeling Prep API and stored in the database. The tool will:
- Identify which changes apply to tickers in your configuration
- Create a backup of config.toml before applying changes
- Add comments showing the old ticker and change date
- Mark changes as applied in the database to avoid reprocessing

### Using the Justfile

The project includes a `justfile` with common development tasks. If you have `just` installed:

```bash
# List all available commands
just --list

# Common tasks
just build          # Build the project
just test           # Run all tests
just fmt            # Format all code
just lint           # Run clippy linter
just quality        # Run all quality checks (fmt, lint, deny)

# Application commands
just rates                      # Fetch current exchange rates
just rates-historical 2024-01-01 2024-12-31  # Fetch historical rates
just marketcaps                 # Fetch and export market caps
just compare 2025-01-01 2025-02-01  # Compare two dates
just ytd                        # Year-to-date comparison

# Database commands
just db             # Open SQLite shell
just db-stats       # Show database statistics
```

### Code Formatting

After making code changes, always run the Rust formatter to ensure code style consistency:

```bash
# Format all code in the project (run from within nix develop)
nix develop --command cargo fmt --all
```

### Dependency and License Checks

After making changes, especially to dependencies, run `cargo-deny` to check for issues:

```bash
# Run cargo-deny checks (run from within nix develop)
nix develop --command cargo deny check
```

## API Rate Limits and Error Handling

- **FMP API**: 300 requests per minute (enforced via semaphore)
- Automatic retry logic for transient failures
- Progress bars for long-running operations
- Comprehensive error messages with anyhow

## CLI Commands

The application supports these main commands:

### Data Fetching
- `MarketCaps` (default) - Fetch and update market cap data
- `ExportCombined` - Export combined market cap report to CSV
- `ExportRates` - Export exchange rates to CSV
- `fetch-historical-exchange-rates` - Backfill historical exchange rates for a date range
- `FetchHistoricalMarketCaps` - Fetch historical yearly data
- `FetchMonthlyHistoricalMarketCaps` - Fetch historical monthly data
- `fetch-specific-date-market-caps` - Fetch market caps for a specific date

### Basic Comparison
- `compare-market-caps` - Compare market caps between two dates
- `generate-charts` - Generate visualization charts from comparison data

### Advanced Comparison
- `trend-analysis` - Multi-date trend analysis (compare more than 2 dates)
- `compare-yoy` - Year-over-Year comparison
- `compare-qoq` - Quarter-over-Quarter comparison
- `compare-rolling` - Rolling period comparison (30d, 90d, 1y, custom)
- `compare-benchmark` - Compare against S&P 500, MSCI indices
- `compare-peer-groups` - Compare predefined industry peer groups

### Utilities
- `list-available-dates` - List dates with available market cap data
- `list-peer-groups` - List predefined peer groups with tickers
- `ListCurrencies` - List all available currencies
- `check-symbol-changes` - Check for ticker symbol changes
- `apply-symbol-changes` - Apply pending symbol changes to config


