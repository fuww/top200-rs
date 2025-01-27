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

## Database

Check the data in the database with:

```sh
sqlite3 top200.db < tests/market_caps_totals_per_month.sql
sqlite3 top200.db < tests/market_caps_totals_per_year.sql
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
