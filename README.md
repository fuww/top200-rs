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
