mod api;
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
mod monthly_marketcap_comparison;
mod ticker_details;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
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
    /// Export monthly market cap comparison to CSV
    ExportMonthlyComparison,
    /// Export exchange rates to CSV
    ExportRates,
    /// Fetch historical market caps
    FetchHistoricalMarketCaps { start_year: i32, end_year: i32 },
    /// Fetch monthly historical market caps
    FetchMonthlyHistoricalMarketCaps { start_year: i32, end_year: i32 },
    /// Add a currency
    AddCurrency { code: String, name: String },
    /// List currencies
    ListCurrencies,
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
        Some(Commands::ExportCombined) => marketcaps::export_market_caps(&pool).await?,
        Some(Commands::ExportMonthlyComparison) => {
            monthly_marketcap_comparison::export_monthly_comparison_csv(&pool).await?
        }
        Some(Commands::ListUs) => details_us_polygon::list_details_us(&pool).await?,
        Some(Commands::ListEu) => details_eu_fmp::list_details_eu(&pool).await?,
        Some(Commands::ExportRates) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            exchange_rates::export_exchange_rates_csv(&fmp_client, &pool).await?;
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
        None => {
            marketcaps::export_market_caps(&pool).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
