// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod advanced_comparisons;
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
mod nats;
mod specific_date_marketcaps;
mod symbol_changes;
mod ticker_details;
mod utils;
mod visualizations;
mod web;

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
    /// Multi-date trend analysis (compare more than 2 dates)
    TrendAnalysis {
        /// Dates to compare (YYYY-MM-DD format, comma-separated)
        #[arg(long, value_delimiter = ',')]
        dates: Vec<String>,
    },
    /// Year-over-Year (YoY) comparison
    CompareYoy {
        /// Reference date (YYYY-MM-DD format)
        #[arg(long)]
        date: String,
        /// Number of years to compare (default: 3)
        #[arg(long, default_value = "3")]
        years: i32,
    },
    /// Quarter-over-Quarter (QoQ) comparison
    CompareQoq {
        /// Reference date (YYYY-MM-DD format)
        #[arg(long)]
        date: String,
        /// Number of quarters to compare (default: 4)
        #[arg(long, default_value = "4")]
        quarters: i32,
    },
    /// Rolling period comparison (30-day, 90-day, 1-year windows)
    CompareRolling {
        /// Reference date (YYYY-MM-DD format)
        #[arg(long)]
        date: String,
        /// Rolling period: 30d, 90d, 180d, 1y, or custom number of days
        #[arg(long, default_value = "30d")]
        period: String,
    },
    /// Compare against a benchmark (S&P 500, MSCI indices)
    CompareBenchmark {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        /// Benchmark to compare against: sp500, msci, or a custom ticker
        #[arg(long, default_value = "sp500")]
        benchmark: String,
    },
    /// Compare peer groups (luxury, sportswear, fast fashion, etc.)
    ComparePeerGroups {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        /// Peer groups to compare (comma-separated). Leave empty for all groups.
        /// Available: luxury, sportswear, fast-fashion, department-stores, value-retail, footwear, e-commerce, asian-fashion
        #[arg(long, value_delimiter = ',')]
        groups: Option<Vec<String>>,
    },
    /// List available dates for comparison (from output directory)
    ListAvailableDates,
    /// List predefined peer groups
    ListPeerGroups,
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
    /// Start the web server
    Serve {
        /// Port to bind to
        #[arg(long, default_value = "3000")]
        port: u16,
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
            compare_marketcaps::compare_market_caps(&from, &to).await?;
        }
        Some(Commands::GenerateCharts { from, to }) => {
            visualizations::generate_all_charts(&from, &to).await?;
        }
        Some(Commands::TrendAnalysis { dates }) => {
            if dates.len() < 2 {
                anyhow::bail!("At least 2 dates are required for trend analysis");
            }
            advanced_comparisons::multi_date_comparison(&pool, dates).await?;
        }
        Some(Commands::CompareYoy { date, years }) => {
            advanced_comparisons::compare_yoy(&pool, &date, years).await?;
        }
        Some(Commands::CompareQoq { date, quarters }) => {
            advanced_comparisons::compare_qoq(&pool, &date, quarters).await?;
        }
        Some(Commands::CompareRolling { date, period }) => {
            let rolling_period = match period.to_lowercase().as_str() {
                "30d" => advanced_comparisons::RollingPeriod::Days30,
                "90d" => advanced_comparisons::RollingPeriod::Days90,
                "180d" => advanced_comparisons::RollingPeriod::Days180,
                "1y" | "1year" | "365d" => advanced_comparisons::RollingPeriod::Year1,
                _ => {
                    // Try to parse as number of days
                    let days: i64 = period
                        .trim_end_matches('d')
                        .parse()
                        .map_err(|_| anyhow::anyhow!(
                            "Invalid period '{}'. Use: 30d, 90d, 180d, 1y, or a number of days (e.g., 45d)",
                            period
                        ))?;
                    advanced_comparisons::RollingPeriod::Custom(days)
                }
            };
            advanced_comparisons::compare_rolling(&pool, &date, rolling_period).await?;
        }
        Some(Commands::CompareBenchmark {
            from,
            to,
            benchmark,
        }) => {
            let bench = match benchmark.to_lowercase().as_str() {
                "sp500" | "s&p500" | "spy" => advanced_comparisons::Benchmark::SP500,
                "msci" | "msci_world" | "urth" => advanced_comparisons::Benchmark::MSCI,
                _ => advanced_comparisons::Benchmark::Custom(benchmark),
            };
            advanced_comparisons::compare_with_benchmark(&pool, &from, &to, bench).await?;
        }
        Some(Commands::ComparePeerGroups { from, to, groups }) => {
            advanced_comparisons::compare_peer_groups(&pool, &from, &to, groups).await?;
        }
        Some(Commands::ListAvailableDates) => {
            let dates = advanced_comparisons::get_available_dates()?;
            if dates.is_empty() {
                println!("No market cap data files found in output/ directory.");
                println!("Run 'fetch-specific-date-market-caps YYYY-MM-DD' to fetch data.");
            } else {
                println!("Available dates for comparison ({} found):", dates.len());
                for date in dates {
                    println!("  {}", date);
                }
            }
        }
        Some(Commands::ListPeerGroups) => {
            let groups = advanced_comparisons::get_predefined_peer_groups();
            println!("Predefined Peer Groups:");
            println!();
            for group in groups {
                println!("  {} ({} tickers)", group.name, group.tickers.len());
                if let Some(desc) = &group.description {
                    println!("    {}", desc);
                }
                println!("    Tickers: {}", group.tickers.join(", "));
                println!();
            }
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
        Some(Commands::Serve { port }) => {
            // Load configuration
            let config = config::load_config()?;

            // Initialize WorkOS client
            let workos_api_key = env::var("WORKOS_API_KEY").expect("WORKOS_API_KEY must be set");
            let api_key = workos::ApiKey::from(workos_api_key.as_str());
            let workos_client = workos::WorkOs::new(&api_key);

            // Get JWT secret
            let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
                println!(
                    "⚠️  Warning: JWT_SECRET not set, using default (insecure for production!)"
                );
                "default-secret-change-in-production".to_string()
            });

            // Initialize NATS client
            let nats_url = env::var("NATS_URL").unwrap_or_else(|_| {
                println!("⚠️  NATS_URL not set, using default: nats://127.0.0.1:4222");
                "nats://127.0.0.1:4222".to_string()
            });

            let nats_client = nats::create_nats_client(&nats_url).await?;

            // Set up JetStream streams
            nats::setup_streams(&nats_client).await?;

            // Start background worker
            let worker_client = nats_client.clone();
            tokio::spawn(async move {
                if let Err(e) = nats::start_worker(worker_client).await {
                    eprintln!("Worker error: {}", e);
                }
            });

            // Create app state
            let state = web::AppState::new(pool, config, workos_client, jwt_secret, nats_client);

            // Start the web server
            web::server::start_server(state, port).await?;
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
