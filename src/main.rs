// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod api;
mod config;
mod currencies;
mod db;
mod details_eu_fmp;
mod details_us_polygon;
mod exchange_rates;
mod models;
mod utils;
mod viz;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
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
    /// Add a currency
    AddCurrency { code: String, name: String },
    /// List currencies
    ListCurrencies,
    /// Generate heatmap
    GenerateHeatmap,
    /// List top 100
    ListTop100,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::ExportUs) => details_us_polygon::export_details_us_csv().await?,
        Some(Commands::ExportEu) => details_eu_fmp::export_details_eu_csv().await?,
        Some(Commands::ExportCombined) => {
            details_us_polygon::export_details_us_csv().await?;
            details_eu_fmp::export_details_eu_csv().await?;
            marketcaps::marketcaps().await?;
        }
        Some(Commands::ListUs) => details_us_polygon::list_details_us().await?,
        Some(Commands::ListEu) => details_eu_fmp::list_details_eu().await?,
        Some(Commands::ExportRates) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            exchange_rates::export_exchange_rates_csv(&fmp_client).await?;
        }
        Some(Commands::AddCurrency { code, name }) => {
            let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db".to_string());
            let pool = db::create_db_pool(&db_url).await?;
            currencies::insert_currency(&pool, &code, &name).await?;
            println!("Added currency: {} ({})", name, code);
        }
        Some(Commands::ListCurrencies) => {
            let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db".to_string());
            let pool = db::create_db_pool(&db_url).await?;
            let currencies = currencies::list_currencies(&pool).await?;
            for (code, name) in currencies {
                println!("{}: {}", code, name);
            }
        }
        Some(Commands::GenerateHeatmap) => {
            generate_heatmap_from_latest()?;
        }
        Some(Commands::ListTop100) => {
            output_top_100_active()?;
        }
        None => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            exchange_rates::export_exchange_rates_csv(&fmp_client).await?;
        }
    }

    Ok(())
}

/// Read CSV file and return records with market cap in EUR
fn read_csv_with_market_cap(file_path: &std::path::Path) -> Result<Vec<(f64, Vec<String>)>> {
    let mut rdr = csv::Reader::from_path(file_path)?;
    let mut results = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let market_cap_str = record.get(2).unwrap_or("0");
        let market_cap: f64 = market_cap_str.parse().unwrap_or(0.0);
        let currency = record.get(3).unwrap_or("EUR");

        // Convert market cap to EUR if necessary
        let market_cap_eur = if currency == "EUR" {
            market_cap
        } else if currency == "USD" {
            market_cap * 0.85 // Approximate USD to EUR conversion
        } else {
            market_cap // Default to original value if currency is unknown
        };

        results.push((market_cap_eur, record.iter().map(|s| s.to_string()).collect()));
    }

    // Sort by market cap in descending order
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

fn output_top_100_active() -> Result<()> {
    let latest_file = find_latest_file("output/eu_marketcaps_*.csv")?;
    let results = read_csv_with_market_cap(&latest_file)?;

    // Filter active companies and take top 100
    let active_results: Vec<_> = results
        .iter()
        .filter(|(_, record)| record[6] == "true") // Active status is at index 6
        .take(100)
        .collect();

    println!("Top 100 Active Companies by Market Cap:");
    println!("Rank\tMarket Cap (EUR)\tTicker\tName");
    for (i, (market_cap, record)) in active_results.iter().enumerate() {
        println!(
            "{}\t{:.2}\t{}\t{}",
            i + 1,
            market_cap,
            record[0], // Ticker
            record[1]  // Name
        );
    }

    Ok(())
}

fn generate_heatmap_from_latest() -> Result<()> {
    let latest_file = find_latest_file("output/eu_marketcaps_*.csv")?;
    let results = read_csv_with_market_cap(&latest_file)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let output_path = format!("output/heatmap_{}.html", timestamp);

    generate_market_heatmap(&results, &output_path)?;
    println!("✅ Heatmap generated at: {}", output_path);

    Ok(())
}

fn generate_market_heatmap(results: &[(f64, Vec<String>)], output_path: &str) -> Result<()> {
    let stocks: Vec<viz::StockData> = results
        .iter()
        .map(|(market_cap, record)| viz::StockData {
            symbol: record[0].clone(), // Ticker is at index 0
            market_cap_eur: *market_cap,
            employees: record[11].clone(), // Employees is at index 11
        })
        .collect();

    viz::create_market_heatmap(stocks, output_path)
}

fn find_latest_file(pattern: &str) -> Result<std::path::PathBuf> {
    use glob::glob;
    let paths: Vec<std::path::PathBuf> = glob(pattern)?.filter_map(|entry| entry.ok()).collect();

    let latest_file = paths
        .iter()
        .max_by_key(|path| path.metadata().unwrap().modified().unwrap())
        .ok_or_else(|| {
            anyhow::anyhow!("No files matching '{}' found in output directory", pattern)
        })?;

    Ok(latest_file.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
}
