<!--
SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>

SPDX-License-Identifier: AGPL-3.0-only
-->

## Top200-RS — Product and Technical Specification

### Overview

Top200-RS collects, stores, analyzes, and visualizes market capitalization data for a curated list of tickers. It pulls fundamentals from external providers, normalizes to EUR and USD, persists to SQLite, exports CSV datasets, compares two snapshots, and generates publication-ready SVG visualizations.

### Goals

- **Daily data collection** with reproducible outputs and artifacts.
- **Historical snapshots** by specific date, monthly and yearly ranges.
- **Comparable outputs** to analyze changes (absolute, percentage, rank, market share).
- **Visual storytelling** via charts for summaries and insights.
- **Reproducible build/runtime** using Nix and CI parity with local commands.

### Non-Goals

- Real-time intraday updates (batch/snapshot oriented).
- Distributed storage; a single SQLite database is sufficient.
- Complex portfolio analytics (focus is market-cap snapshots and comparisons).

## Architecture

### Components

- **Rust CLI** (`src/main.rs`): entry-point with subcommands to run data collection, export, comparison, and visualization tasks.
- **SQLite** (`data.db` by default): persistent store for market caps, ticker details, currencies, and forex rates.
- **External APIs**:
  - FMP (Financial Modeling Prep) — market caps, currencies, exchange rates.
  - Polygon — US market details (optional path).
- **Nix Flake** (`flake.nix`): reproducible toolchain and apps mirroring GitHub Actions steps.
- **GitHub Actions** (`.github/workflows/*.yml`): scheduled and manual workflows for collection and notifications.

### Data Flow (typical daily run)

1. Load config and environment (dotenv, `DATABASE_URL`).
2. Update currencies and forex rates from FMP.
3. Fetch details for each configured ticker; compute EUR/USD market caps; write to DB with a single consistent timestamp.
4. Export combined CSV and top 100 active CSV.
5. Optionally run specific-date snapshot(s), then compare two dates and generate visualizations.

### Environment Variables

- **Required at runtime**
  - `FINANCIALMODELINGPREP_API_KEY`: FMP API key (primary key used throughout). Some paths may also read `FMP_API_KEY` as a fallback.
- **Optional**
  - `POLYGON_API_KEY`: for US market details path.
  - `DATABASE_URL`: defaults to `sqlite:data.db`.
  - Email notifications (CI): `BREVO_API_KEY`, `BREVO_SENDER_EMAIL`, `BREVO_SENDER_NAME`, `NOTIFICATION_RECIPIENTS`.

### Database Model (high-level)

- `market_caps`
  - `ticker`, `name`
  - `market_cap_original`, `original_currency`
  - `market_cap_eur`, `market_cap_usd`
  - `exchange`, `price` (if available), `active`
  - `timestamp` (Unix seconds; one timestamp per batch/snapshot)
- `ticker_details`
  - `ticker`, `description`, `homepage_url`, `employees`
- `currencies`
  - Canonical currency codes and metadata
- `forex_rates`
  - `base_currency`, `target_currency`, `rate`, `timestamp`
- `symbol_changes`
  - Source symbol, new symbol, status, reason, applied flags, timestamps

Migrations are tracked under `migrations/` and applied automatically on startup.

## CLI Specification

### Global

- Executable: `top200-rs`
- All commands respect `DATABASE_URL` and load `.env` at runtime.

### Commands

- **export-combined**
  - Pipeline: update currencies and forex, update market caps for configured tickers, export combined dataset and top 100 active.
  - Output: `output/combined_marketcaps_<timestamp>.csv`, `output/top_100_active_<timestamp>.csv`

- **fetch-specific-date-market-caps <YYYY-MM-DD>**
  - Fetch snapshot for a specific date (UTC 00:00) for all configured tickers; convert EUR/USD; persist and export.
  - Output: `output/marketcaps_<YYYY-MM-DD>_<timestamp>.csv`

- **compare-market-caps --from <YYYY-MM-DD> --to <YYYY-MM-DD>**
  - Read the latest CSV for each date from `output/`; compute absolute/percentage changes, rank movement, market shares.
  - Output: `output/comparison_<from>_to_<to>_<timestamp>.csv` and summary markdown `output/comparison_<from>_to_<to>_summary_<timestamp>.md`.

- **generate-charts --from <YYYY-MM-DD> --to <YYYY-MM-DD>**
  - Read comparison CSV; generate SVG charts:
    - `..._gainers_losers.svg`
    - `..._market_distribution.svg`
    - `..._rank_movements.svg`
    - `..._summary_dashboard.svg`

- **export-us / export-eu / export-combined**
  - Export regional or combined datasets from the current DB snapshot.

- **export-rates**
  - Update forex rates from FMP into the database.

- **fetch-historical-market-caps <start_year> <end_year>**
  - Yearly snapshots for the range.

- **fetch-monthly-historical-market-caps <start_year> <end_year>**
  - Monthly snapshots for the range.

- **add-currency --code <XXX> --name <NAME>** / **list-currencies**
  - Maintain currency metadata; uses FMP to refresh known currencies first.

- **check-symbol-changes [--config <path>]** / **apply-symbol-changes [--config <path>] [--dry-run] [--auto-apply]**
  - Discover and optionally apply configuration ticker updates; tracks applied changes.

## Nix Flake and Local Apps

### Development Shell

- `nix develop` provides a toolchain with Rust, cargo, sqlx-cli, sqlite, and required system libs.

### Packages

- `packages.default` builds the CLI using oxalica rust overlay and crane.

### Apps (CI-parity wrappers)

- `.#build` — prepares SQLx offline cache if needed and builds in release mode.
- `.#specific-date -- <YYYY-MM-DD>` — runs specific date snapshot.
- `.#compare -- <FROM> <TO>` — runs CSV comparison.
- `.#generate-charts -- <FROM> <TO>` — generates charts for an existing comparison.
- `.#pipeline-today-vs-7days` — end-to-end: specific date (today, 7 days ago), compare, visualize.
- `.#export-combined` — mirrors the daily export step.

Notes:

- The wrappers set SSL cert env vars for HTTPS and respect `DATABASE_URL` (default `sqlite:data.db`).
- `FINANCIALMODELINGPREP_API_KEY` must be present for all FMP-backed commands.

## GitHub Actions Workflows

- `daily-run.yml` — daily combined export, artifact upload, optional Brevo email notification.
- `daily-specific-date.yml` — daily specific date snapshot (UTC), artifact upload, optional email.
- `update-flake.yml` — daily `flake.lock` update via PR.
- `flakehub-publish-tagged.yml` — pushes tagged versions to FlakeHub.
- `nix-ci.yml` — shared Determinate Systems CI scaffold for flake checks.

## Outputs

- CSVs under `output/` with timestamped filenames for reproducibility.
- Comparison outputs include detailed per-ticker metrics and a human-readable summary markdown.
- Visualizations are `SVG` for high-quality embedding.

## Error Handling and Operational Notes

- API keys are validated early; commands fail fast with clear error messages if missing.
- SQL migrations run automatically on startup.
- Exchange rates are sourced from DB for conversions; run `export-rates`/pipeline first in fresh databases.
- CSV discovery uses filename conventions; ensure prior steps ran for the requested dates.

## Performance Considerations

- Batch processing with progress bars; network-bound on external API responses.
- Currency conversions are done in-memory using a rate map to minimize DB round-trips.

## Security Considerations

- Store secrets in environment variables and GitHub Secrets (never commit to VCS).
- CI email step validates config before sending; fails the job if misconfigured to surface issues early.

## Acceptance Criteria (per feature)

- **Specific date snapshot**: running with a valid date creates one CSV in `output/` and persists rows with the correct UNIX timestamp.
- **Combined export**: creates two CSVs (`combined_marketcaps_*.csv`, `top_100_active_*.csv`).
- **Comparison**: produces CSV and summary markdown referencing the two input dates; handles missing tickers gracefully.
- **Visualizations**: generates four SVGs and prints completion messages.
- **Symbol changes**: `check` prints a summary; `apply` respects `--dry-run` and `--auto-apply` and updates DB tracking.

## Future Enhancements

- Parallelize per-ticker fetches with bounded concurrency.
- Additional visualizations (sector breakdowns, currency exposure).
- Pluggable provider backends and rate-limit aware scheduling.
- Optional export to Parquet for downstream analytics.


