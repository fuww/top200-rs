// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod api;
mod compare_marketcaps;
mod config;
mod currencies;
mod db;
mod details_eu_fmp;
mod details_us_polygon;
mod exchange_rates;
mod historical_marketcaps;
mod marketcaps;
mod models;
mod monthly_historical_marketcaps;
mod specific_date_marketcaps;
mod symbol_changes;
mod ticker_details;
mod utils;
mod visualizations;

use anyhow::Result;
use clap::{Parser, Subcommand};
// use sqlx::sqlite::SqlitePool;
use std::env;
use tokio;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Export US market caps to CSV
    ExportUs,
    /// Export EU market caps to CSV
    ExportEu,
    /// Export combined market caps to CSV
    ExportCombined,
    /// List US market caps
    ListUs,
    /// List EU market caps
    ListEu,
    /// Export exchange rates to CSV
    ExportRates,
    /// Fetch historical exchange rates for a date range
    FetchHistoricalExchangeRates {
        /// Start date (YYYY-MM-DD format)
        #[arg(long)]
        from: String,
        /// End date (YYYY-MM-DD format)
        #[arg(long)]
        to: String,
    },
    /// Fetch historical market caps
    FetchHistoricalMarketCaps { start_year: i32, end_year: i32 },
    /// Fetch monthly historical market caps
    FetchMonthlyHistoricalMarketCaps { start_year: i32, end_year: i32 },
    /// Fetch market caps for a specific date
    FetchSpecificDateMarketCaps { date: String },
    /// Add a currency
    AddCurrency { code: String, name: String },
    /// List currencies
    ListCurrencies,
    /// Compare market caps between two dates
    CompareMarketCaps {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    /// Generate visualization charts from comparison data
    GenerateCharts {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
    },
    /// Check for symbol changes that need to be applied
    CheckSymbolChanges {
        /// Path to config.toml file
        #[arg(long, default_value = "config.toml")]
        config: String,
    },
    /// Apply pending symbol changes to configuration
    ApplySymbolChanges {
        /// Path to config.toml file
        #[arg(long, default_value = "config.toml")]
        config: String,
        /// Show what would be changed without applying
        #[arg(long)]
        dry_run: bool,
        /// Automatically apply all non-conflicting changes
        #[arg(long)]
        auto_apply: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db".to_string());
    let pool = db::create_db_pool(&db_url).await?;

    match cli.command {
        Some(Commands::ExportUs) => details_us_polygon::export_details_us_csv(&pool).await?,
        Some(Commands::ExportEu) => details_eu_fmp::export_details_eu_csv(&pool).await?,
        Some(Commands::ExportCombined) => {
            marketcaps::marketcaps(&pool).await?;
        }
        Some(Commands::ListUs) => details_us_polygon::list_details_us(&pool).await?,
        Some(Commands::ListEu) => details_eu_fmp::list_details_eu(&pool).await?,
        Some(Commands::ExportRates) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            exchange_rates::update_exchange_rates(&fmp_client, &pool).await?;
        }
        Some(Commands::FetchHistoricalExchangeRates { from, to }) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            exchange_rates::fetch_historical_exchange_rates(&fmp_client, &pool, &from, &to).await?;
        }
        Some(Commands::FetchHistoricalMarketCaps {
            start_year,
            end_year,
        }) => {
            historical_marketcaps::fetch_historical_marketcaps(&pool, start_year, end_year).await?;
        }
        Some(Commands::FetchMonthlyHistoricalMarketCaps {
            start_year,
            end_year,
        }) => {
            monthly_historical_marketcaps::fetch_monthly_historical_marketcaps(
                &pool, start_year, end_year,
            )
            .await?;
        }
        Some(Commands::FetchSpecificDateMarketCaps { date }) => {
            specific_date_marketcaps::fetch_specific_date_marketcaps(&pool, &date).await?;
        }
        Some(Commands::AddCurrency { code, name }) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            currencies::update_currencies(&fmp_client, &pool).await?;
            println!("✅ Currencies updated from FMP API");

            // Also add the manually specified currency
            currencies::insert_currency(&pool, &code, &name).await?;
            println!("✅ Added currency: {} ({})", name, code);
        }
        Some(Commands::ListCurrencies) => {
            let currencies = currencies::list_currencies(&pool).await?;
            for (code, name) in currencies {
                println!("{}: {}", code, name);
            }
        }
        Some(Commands::CompareMarketCaps { from, to }) => {
            compare_marketcaps::compare_market_caps(&pool, &from, &to).await?;
        }
        Some(Commands::GenerateCharts { from, to }) => {
            visualizations::generate_all_charts(&from, &to).await?;
        }
        Some(Commands::CheckSymbolChanges { config }) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .or_else(|_| env::var("FMP_API_KEY"))
                .expect("FINANCIALMODELINGPREP_API_KEY or FMP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);

            // Fetch and store latest symbol changes
            symbol_changes::fetch_and_store_symbol_changes(&pool, &fmp_client).await?;

            // Check which changes apply to our config
            let report = symbol_changes::check_ticker_updates(&pool, &config).await?;
            symbol_changes::print_symbol_change_report(&report);
        }
        Some(Commands::ApplySymbolChanges {
            config,
            dry_run,
            auto_apply,
        }) => {
            // Check which changes apply to our config
            let report = symbol_changes::check_ticker_updates(&pool, &config).await?;
            symbol_changes::print_symbol_change_report(&report);

            if report.applicable_changes.is_empty() {
                println!("\nNo applicable changes to apply.");
            } else if auto_apply || dry_run {
                // Apply all applicable changes
                symbol_changes::apply_ticker_updates(
                    &pool,
                    &config,
                    report.applicable_changes,
                    dry_run,
                )
                .await?;
            } else {
                // Interactive mode - ask user to confirm
                println!("\nFound {} applicable changes. Run with --auto-apply to apply them or --dry-run to preview.", 
                    report.applicable_changes.len());
            }
        }
        None => {
            marketcaps::marketcaps(&pool).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
