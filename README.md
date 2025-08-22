<!--
SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>

SPDX-License-Identifier: AGPL-3.0-only
-->

[![CI](https://github.com/javdl/top200-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/javdl/top200-rs/actions/workflows/ci.yml)[![Daily Data Collection](https://github.com/javdl/top200-rs/actions/workflows/daily-run.yml/badge.svg)](https://github.com/javdl/top200-rs/actions/workflows/daily-run.yml)

# Top 200 Market Cap Tracker

A Rust application that tracks and analyzes market capitalization data for top companies.

## Features

- Fetches and updates currency exchange rates
- Retrieves market cap data from financial APIs
- Stores data in SQLite database
- Exports data to CSV format
- Compares market cap data between dates with detailed analytics
- Handles rate limiting and retries

## Getting Started

### Prerequisites

- Rust toolchain
- Nix package manager

### Installation

```bash
git clone https://github.com/javdl/top200-rs.git
cd top200-rs
```

### Environment Setup

Create a `.env` file with your API keys:

```env
FMP_API_KEY=your_api_key_here
```

## Usage

Run the application:

```bash
nix develop 
cargo run
```

Windsurf should use:

```bash
nix develop --command cargo run
```

Get a list of available commands:

```bash
cargo run -- --help
```

### Example Commands

Fetch market caps for a specific date:

```bash
# Fetch market caps for August 1, 2025
cargo run -- fetch-specific-date-market-caps 2025-08-01

# This will:
# - Fetch market cap data for all configured tickers
# - Retrieve exchange rates from the database
# - Export data to output/marketcaps_2025-08-01_YYYYMMDD_HHMMSS.csv
```

Compare market caps between two dates:

```bash
# Compare market caps between July 1 and August 1, 2025
cargo run -- compare-market-caps --from 2025-07-01 --to 2025-08-01

# This will generate:
# - Detailed comparison CSV with all metrics
# - Summary report in Markdown format
# - Analysis includes:
#   * Percentage and absolute changes
#   * Ranking changes
#   * Market share shifts
#   * Top gainers/losers

# One-liner to fetch and compare year-end 2024 with today
cargo run -- fetch-specific-date-market-caps 2024-12-31 && cargo run -- fetch-specific-date-market-caps $(date +%Y-%m-%d) && cargo run -- compare-market-caps --from 2024-12-31 --to $(date +%Y-%m-%d)
```

Generate visualization charts from comparison data:

```bash
# Generate beautiful SVG charts from comparison data
cargo run -- generate-charts --from 2025-07-01 --to 2025-08-01

# This will create 4 professional visualization charts:
# 1. Top Gainers and Losers bar chart
# 2. Market Cap Distribution donut chart  
# 3. Rank Movements chart
# 4. Market Summary Dashboard

# Output files:
# - output/comparison_YYYY-MM-DD_to_YYYY-MM-DD_gainers_losers.svg
# - output/comparison_YYYY-MM-DD_to_YYYY-MM-DD_market_distribution.svg
# - output/comparison_YYYY-MM-DD_to_YYYY-MM-DD_rank_movements.svg
# - output/comparison_YYYY-MM-DD_to_YYYY-MM-DD_summary_dashboard.svg

# Complete workflow: fetch, compare, and visualize
cargo run -- fetch-specific-date-market-caps 2025-07-01 && \
cargo run -- fetch-specific-date-market-caps 2025-08-01 && \
cargo run -- compare-market-caps --from 2025-07-01 --to 2025-08-01 && \
cargo run -- generate-charts --from 2025-07-01 --to 2025-08-01
```

Track and apply stock ticker symbol changes:

```bash
# Check for symbol changes that affect configured tickers
cargo run -- check-symbol-changes

# Preview changes without applying them
cargo run -- apply-symbol-changes --dry-run

# Automatically apply all non-conflicting symbol changes
cargo run -- apply-symbol-changes --auto-apply

# The tool will:
# - Fetch symbol changes from FMP API (mergers, acquisitions, rebranding)
# - Identify which changes apply to your configured tickers
# - Create a backup of config.toml before applying changes
# - Add comments showing the old ticker and change date
# - Track applied changes to avoid reprocessing
```

Export combined market cap report:

```bash
cargo run -- export-combined
```

Fetch historical data:

```bash
# Yearly data
cargo run -- fetch-historical-market-caps 2023 2025

# Monthly data
cargo run -- fetch-monthly-historical-market-caps 2023 2025
```

## Database Browsing

### Accessing the SQLite Database

The application stores all data in a SQLite database (`data.db`). You can browse and query this database using the `sqlite3` command-line tool.

## Email Notifications

The GitHub workflows can send email notifications when data collection jobs complete successfully. This is useful for staying informed about daily market cap updates.

### Configuration

Email notifications are configured through environment variables:

```env
# Required for email functionality
BREVO_API_KEY=your_brevo_api_key_here
BREVO_SENDER_EMAIL=noreply@example.com
NOTIFICATION_RECIPIENTS=user1@example.com,user2@example.com

# Optional
BREVO_SENDER_NAME=Top200-RS Notifier
```

### Features

- **Multiple Recipients**: Send notifications to multiple email addresses (comma-separated)
- **Artifact Links**: Each email includes direct links to download generated CSV files
- **Run Details**: Workflow information, commit SHA, and timestamps
- **Fallback Support**: Maintains backward compatibility with existing setups

### GitHub Configuration

Set these in your repository's Settings > Secrets and variables:

**Secrets:**
- `BREVO_API_KEY`: Your Brevo transactional email API key
- `NOTIFICATION_RECIPIENTS`: Comma-separated list of email addresses

**Variables:**
- `BREVO_SENDER_EMAIL`: Email address to send from
- `BREVO_SENDER_NAME`: Display name for notifications

See [docs/email-notifications.md](docs/email-notifications.md) for detailed configuration instructions.

#### Basic Database Commands

```bash
# Open the database
sqlite3 data.db

# List all tables
.tables

# Show table schemas
.schema market_caps
.schema ticker_details
.schema currencies
.schema forex_rates

# Exit sqlite3
.quit
```

#### Viewing Table Structures

```bash
# View all columns in the market_caps table
sqlite3 data.db ".schema market_caps"

# View all columns in the ticker_details table
sqlite3 data.db ".schema ticker_details"
```

#### Looking Up Company Information

To find information about a specific company (e.g., MYT/MYTE):

```bash
# Search for a ticker (case-insensitive)
sqlite3 data.db "SELECT DISTINCT ticker, name FROM market_caps WHERE ticker LIKE '%MYT%';"

# Get recent market cap data for a specific company (e.g., MYTE)
sqlite3 data.db -header -column "
  SELECT 
    ticker, 
    name, 
    market_cap_usd, 
    exchange, 
    datetime(timestamp, 'unixepoch') as date 
  FROM market_caps 
  WHERE ticker = 'MYTE' 
  ORDER BY timestamp DESC 
  LIMIT 10;"

# Get company details from ticker_details table
sqlite3 data.db -header -column "
  SELECT * FROM ticker_details 
  WHERE ticker = 'MYTE';"

# Get the latest market cap data with all fields
sqlite3 data.db -header -column "
  SELECT 
    ticker,
    name,
    market_cap_original,
    original_currency,
    market_cap_eur,
    market_cap_usd,
    exchange,
    price,
    employees,
    revenue_usd,
    pe_ratio,
    datetime(timestamp, 'unixepoch') as date
  FROM market_caps 
  WHERE ticker = 'MYTE' 
  ORDER BY timestamp DESC 
  LIMIT 1;"
```

#### Useful Queries

```bash
# List all companies with their latest market caps
sqlite3 data.db -header -column "
  SELECT 
    ticker, 
    name, 
    market_cap_usd/1000000000 as market_cap_billions_usd,
    exchange
  FROM market_caps 
  WHERE timestamp = (SELECT MAX(timestamp) FROM market_caps)
  ORDER BY market_cap_usd DESC;"

# Find top 10 companies by market cap
sqlite3 data.db -header -column "
  SELECT 
    ticker, 
    name, 
    ROUND(market_cap_usd/1000000000, 2) as market_cap_billions
  FROM market_caps 
  WHERE timestamp = (SELECT MAX(timestamp) FROM market_caps)
  ORDER BY market_cap_usd DESC 
  LIMIT 10;"

# Track market cap changes over time for a company
sqlite3 data.db -header -column "
  SELECT 
    datetime(timestamp, 'unixepoch') as date,
    market_cap_usd,
    price
  FROM market_caps 
  WHERE ticker = 'MYTE' 
  ORDER BY timestamp DESC 
  LIMIT 30;"

# List all available currencies
sqlite3 data.db -header -column "SELECT * FROM currencies;"

# View recent exchange rates
sqlite3 data.db -header -column "
  SELECT 
    base_currency,
    target_currency,
    rate,
    datetime(timestamp, 'unixepoch') as date
  FROM forex_rates 
  ORDER BY timestamp DESC 
  LIMIT 20;"
```

#### Export Query Results to CSV

```bash
# Export company data to CSV
sqlite3 -header -csv data.db "
  SELECT * FROM market_caps 
  WHERE ticker = 'MYTE' 
  ORDER BY timestamp DESC;" > myte_data.csv

# Export all latest market caps to CSV
sqlite3 -header -csv data.db "
  SELECT * FROM market_caps 
  WHERE timestamp = (SELECT MAX(timestamp) FROM market_caps)
  ORDER BY market_cap_usd DESC;" > latest_market_caps.csv
```

## Development

### Resources

- [Rust Windsurf Transformation Guide](https://neoexogenesis.com/posts/rust-windsurf-transformation/)

### TODO

- [ ] Add a command to test with a subset of tickers
- [ ] Add more comprehensive error handling
- [ ] Improve rate limiting strategy

## License

This project is licensed under the AGPL-3.0 License - see the LICENSE file for details.

## Changelog

- 2025-01: Added Windsurf support and documentation
- 2023-10: Converted exchange prefixes to FMP format (EPA: -> .PA, BME: -> .MC, VTX: -> .SW, ETR: -> .DE, LON: -> .L, BIT: -> .MI, STO: -> .ST, TYO: -> .T, HKG: -> .HK, BVMF: -> .SA, TSE: -> .TO)
- 2023-09: TED.L (Ted Baker) - Delisted after being acquired by Authentic Brands Group
- 2023-07: DSW (Designer Shoe Warehouse) - Changed name and ticker to Designer Brands Inc. (DBI)
- 2023-06: SGP.L (Supergroup) - Changed name and ticker to Superdry (SDRY.L)
- 2023-05: Company name changes (COH -> TPR Tapestry, KORS -> CPRI Capri Holdings, LB -> BBWI Bath & Body Works)
- 2023-03: Fixed currency conversion for ILA to ILS (Israeli Shekel)
- 2023-01: Initial release with basic market cap tracking functionality
- 2021-06: HGTX3.SA (Cia Hering) - Merged with Grupo Soma, now part of SOMA3.SA
- 2021-01: TIF (Tiffany & Co.) - Delisted after being acquired by LVMH
- 2020-12: FRAN (Francesca's) - Filed for bankruptcy and was delisted
- 2020-07: ASNA (Ascena Retail Group) - Filed for bankruptcy and was delisted
- 2020-03: HBC.TO (Hudson's Bay) - Delisted after going private
- 2020-01: JCP (J.C. Penney) - Filed for bankruptcy and was delisted
- 2019-07: GWI1.DE (Gerry Weber) - Filed for insolvency, restructured and now trades as GWI2.DE
- 2019-05: SPD.L (Sports Direct) - Changed name and ticker to Frasers Group (FRAS.L)
- 2018-05: YNAP.MI (YOOX Net-a-Porter Group) - Delisted after being fully acquired by Richemont

Note: Some tickers might have data provider issues:

- ALPA4.SA (Alpargatas) - Still trades on Brazilian exchange as ALPA4.SA and ALPA3.SA
- DLTI.TA - Currency code should be ILS instead of ILA
