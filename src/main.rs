// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod api;
mod config;
mod models;
mod utils;
mod viz;

use anyhow::Result;
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use csv::Writer;
use dotenvy::dotenv;
use glob::glob;
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

pub use utils::convert_currency;

#[derive(Parser)]
#[command(name = "top200-rs")]
#[command(about = "A CLI tool for stock market data analysis", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export combined US & non-US stock marketcaps to CSV & generate treemap
    ExportCombined,
    /// Export currency exchange rates to CSV
    ExportRates,
    /// List US stock marketcaps (Polygon API)
    ListUs,
    /// List EU stock marketcaps
    ListEu,
    /// Export US stock marketcaps to CSV
    ExportUs,
    /// Export EU stock marketcaps to CSV
    ExportEu,
    /// Generate Market Heatmap from latest top 100
    GenerateHeatmap,
    /// Output top 100 active tickers
    ListTop100,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::ExportCombined) {
        Commands::ExportCombined => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            export_details_combined_csv(&fmp_client).await?;
        }
        Commands::ExportRates => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            export_exchange_rates_csv(&fmp_client).await?;
        }
        Commands::ListUs => list_details_us().await?,
        Commands::ListEu => list_details_eu().await?,
        Commands::ExportUs => export_details_us_csv().await?,
        Commands::ExportEu => export_details_eu_csv().await?,
        Commands::GenerateHeatmap => generate_heatmap_from_latest()?,
        Commands::ListTop100 => output_top_100_active()?,
    }

    Ok(())
}

async fn export_details_eu_csv() -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.non_us_tickers;

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("eu_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Ticker",
        "Company Name",
        "Market Cap",
        "Currency",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
    ])?;

    let rate_map = get_rate_map();

    let mut tasks = Vec::new();

    for ticker in tickers {
        let ticker = ticker.clone();
        let rate_map = rate_map.clone();
        tasks.push(tokio::spawn(async move {
            let details = api::get_details_eu(&ticker, &rate_map).await;
            (ticker, details)
        }));
    }

    for task in tasks {
        let (ticker, details) = task.await?;
        match details {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details
                        .market_cap
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details
                        .extra
                        .get("exchange")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    &details
                        .extra
                        .get("price")
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .revenue_usd
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .working_capital_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .quick_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .debt_equity_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    &ticker, "", "", "", "", "", "", &error_msg, "", "", "", "", "", "", "", "", "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", csv_path.display());

    Ok(())
}

async fn export_details_us_csv() -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.us_tickers;
    let api_key = env::var("POLYGON_API_KEY").expect("POLYGON_API_KEY must be set");
    let client = api::PolygonClient::new(api_key);
    let date = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("us_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Ticker",
        "Company Name",
        "Market Cap",
        "Currency",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
    ])?;

    for (i, ticker) in tickers.iter().enumerate() {
        println!(
            "\nFetching the marketcap for {} ({}/{}) ⌛️",
            ticker,
            i + 1,
            tickers.len()
        );
        match client.get_details(ticker, date).await {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details
                        .market_cap
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .revenue_usd
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .working_capital_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .quick_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .debt_equity_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    &ticker, "", "", "", "", &error_msg, "", "", "", "", "", "", "", "", "", "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", csv_path.display());

    Ok(())
}

async fn list_details_us() -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.us_tickers;
    let api_key = env::var("POLYGON_API_KEY").expect("POLYGON_API_KEY must be set");
    let client = api::PolygonClient::new(api_key);
    let date = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();

    for (i, ticker) in tickers.iter().enumerate() {
        println!(
            "\nFetching the marketcap for {} ({}/{}) ⌛️",
            ticker,
            i + 1,
            tickers.len()
        );
        match client.get_details(ticker, date).await {
            Ok(details) => {
                println!("Company: {}", details.name.unwrap_or_default());
                if let Some(market_cap) = details.market_cap {
                    println!(
                        "Market Cap: {} {}",
                        details.currency_symbol.unwrap_or_default(),
                        market_cap
                    );
                }
                println!("Active: {}", details.active.unwrap_or_default());
                println!("---");
            }
            Err(e) => eprintln!("Error fetching details for {}: {}", ticker, e),
        }
    }

    Ok(())
}

async fn list_details_eu() -> Result<()> {
    let config = config::load_config()?;
    let tickers = config.non_us_tickers;

    let rate_map = get_rate_map();

    let mut tasks = Vec::new();

    for ticker in tickers {
        let ticker = ticker.clone();
        let rate_map = rate_map.clone();
        tasks.push(tokio::spawn(async move {
            let details = api::get_details_eu(&ticker, &rate_map).await;
            (ticker, details)
        }));
    }

    for task in tasks {
        let (ticker, details) = task.await?;
        match details {
            Ok(details) => {
                println!("Company: {}", details.name.unwrap_or_default());
                if let Some(market_cap) = details.market_cap {
                    println!(
                        "Market Cap: {} {}",
                        details.currency_symbol.unwrap_or_default(),
                        market_cap
                    );
                }
                println!("Active: {}", details.active.unwrap_or_default());
                println!("---");
            }
            Err(e) => eprintln!("Error fetching details for {}: {}", ticker, e),
        }
    }

    Ok(())
}

async fn export_details_combined_csv(fmp_client: &api::FMPClient) -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

    // First fetch exchange rates
    println!("Fetching current exchange rates...");
    let exchange_rates = match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            println!("✅ Exchange rates fetched");
            rates
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to fetch exchange rates: {}", e));
        }
    };

    // Create a map of currency pairs to rates
    let mut rate_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for rate in exchange_rates {
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            rate_map.insert(name, price);
        }
    }

    // Convert exchange prefixes to FMP format
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/combined_marketcaps_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = csv::Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
        "Timestamp",
    ])?;

    // Create a rate_map Arc for sharing between tasks
    let rate_map = Arc::new(rate_map);
    let total_tickers = tickers.len();

    // Process tickers in parallel with progress tracking
    let mut results = Vec::new();
    let progress = indicatif::ProgressBar::new(total_tickers as u64);
    progress.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Create chunks of tickers to process in parallel
    // Process 50 tickers at a time to stay well within rate limits
    for chunk in tickers.chunks(50) {
        let chunk_futures = chunk.iter().map(|ticker| {
            let rate_map = rate_map.clone();
            let ticker = ticker.to_string();
            let progress = progress.clone();

            async move {
                let result = match fmp_client.get_details(&ticker, &rate_map).await {
                    Ok(details) => {
                        let original_market_cap = details.market_cap.unwrap_or(0.0);
                        let currency = details.currency_symbol.clone().unwrap_or_default();
                        let eur_market_cap = crate::utils::convert_currency(
                            original_market_cap,
                            &currency,
                            "EUR",
                            &rate_map,
                        );
                        let usd_market_cap = crate::utils::convert_currency(
                            original_market_cap,
                            &currency,
                            "USD",
                            &rate_map,
                        );

                        Some((
                            eur_market_cap,
                            vec![
                                details.ticker,
                                details.name.unwrap_or_default(),
                                original_market_cap.round().to_string(),
                                currency,
                                eur_market_cap.round().to_string(),
                                usd_market_cap.round().to_string(),
                                details
                                    .extra
                                    .get("exchange")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                details
                                    .extra
                                    .get("price")
                                    .map(|v| v.to_string())
                                    .unwrap_or_default(),
                                details.active.map(|a| a.to_string()).unwrap_or_default(),
                                details.description.unwrap_or_default(),
                                details.homepage_url.unwrap_or_default(),
                                details.employees.unwrap_or_default(),
                                details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                                details
                                    .revenue_usd
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details
                                    .working_capital_ratio
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details
                                    .quick_ratio
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details.eps.map(|r| r.to_string()).unwrap_or_default(),
                                details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                                details
                                    .debt_equity_ratio
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details.roe.map(|r| r.to_string()).unwrap_or_default(),
                                details.timestamp.unwrap_or_default(),
                            ],
                        ))
                    }
                    Err(e) => {
                        eprintln!("Error fetching data for {}: {}", ticker, e);
                        None
                    }
                };
                progress.inc(1);
                result
            }
        });

        // Wait for the current chunk to complete
        let chunk_results: Vec<_> = futures::future::join_all(chunk_futures).await;
        results.extend(chunk_results.into_iter().flatten());
    }

    progress.finish_with_message("Data collection complete");

    // Sort by market cap (EUR)
    results.sort_by(|(a_cap, _): &(f64, Vec<String>), (b_cap, _)| {
        b_cap
            .partial_cmp(a_cap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Write all results
    for (_, record) in &results {
        writer.write_record(record)?;
    }
    writer.flush()?;
    println!("✅ Combined market caps written to: {}", filename);

    // Filter active tickers and get top 100
    let top_100_results: Vec<(f64, Vec<String>)> = results
        .iter()
        .filter(|(_, record)| record[8] == "true") // Active column
        .take(100)
        .map(|(cap, record)| (*cap, record.clone()))
        .collect();

    // Generate top 100 CSV
    let top_100_filename = format!("output/top_100_active_{}.csv", timestamp);
    let top_100_file = std::fs::File::create(&top_100_filename)?;
    let mut top_100_writer = csv::Writer::from_writer(top_100_file);

    // Write headers
    top_100_writer.write_record(&[
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
        "Timestamp",
    ])?;

    // Write top 100 records
    for (_, record) in &top_100_results {
        top_100_writer.write_record(record)?;
    }
    top_100_writer.flush()?;
    println!("✅ Top 100 active tickers written to: {}", top_100_filename);

    // Generate market heatmap from top 100
    generate_market_heatmap(&top_100_results, "output/market_heatmap.png")?;
    println!("✅ Market heatmap generated from top 100 active tickers");

    Ok(())
}

async fn export_exchange_rates_csv(fmp_client: &api::FMPClient) -> Result<()> {
    println!("Fetching current exchange rates...");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/exchange_rates_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = csv::Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Symbol",
        "Rate",
        "Change %",
        "Change",
        "Day Low",
        "Day High",
        "Base Currency",
        "Quote Currency",
        "Timestamp",
    ])?;

    match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            for rate in rates {
                // Split the symbol into base and quote currencies (e.g., "EUR/USD" -> ["EUR", "USD"])
                let currencies: Vec<&str> = rate.name.as_deref().unwrap_or("").split('/').collect();
                let (base, quote) = if currencies.len() == 2 {
                    (currencies[0], currencies[1])
                } else {
                    ("", "")
                };

                writer.write_record(&[
                    rate.name.as_deref().unwrap_or(""),
                    &rate
                        .price
                        .map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate
                        .changes_percentage
                        .map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate
                        .change
                        .map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate
                        .day_low
                        .map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate
                        .day_high
                        .map_or_else(|| "".to_string(), |v| v.to_string()),
                    base,
                    quote,
                    &rate.timestamp.to_string(),
                ])?;
            }
            println!("✅ Exchange rates written to CSV");
        }
        Err(e) => {
            eprintln!("Error fetching exchange rates: {}", e);
            writer.write_record(&[
                "ERROR",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &format!("Error: {}", e),
            ])?;
        }
    }

    println!("📝 CSV file created: {}", filename);
    Ok(())
}

#[allow(dead_code)]
async fn export_marketcap_with_progress(tickers: Vec<String>, output_path: &str) -> Result<()> {
    let mut writer = Writer::from_path(output_path)?;

    writer.write_record(&[
        "Ticker",
        "Market Cap",
        "Name",
        "Currency Name",
        "Currency Symbol",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
    ])?;

    let rate_map = get_rate_map();
    let fmp_client = api::FMPClient::new(
        env::var("FINANCIALMODELINGPREP_API_KEY")
            .expect("FINANCIALMODELINGPREP_API_KEY must be set"),
    );

    for ticker in tickers {
        match fmp_client.get_details(&ticker, &rate_map).await {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details
                        .market_cap
                        .map(|m| m.to_string())
                        .unwrap_or_default(),
                    &details.name.unwrap_or_default(),
                    &details.currency_name.unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .revenue_usd
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .working_capital_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details
                        .quick_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details
                        .debt_equity_ratio
                        .map(|r| r.to_string())
                        .unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    &ticker, "", "", "", "", "", "", &error_msg, "", "", "", "", "", "", "", "", "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", output_path);
    Ok(())
}

#[allow(dead_code)]
async fn export_marketcap_to_json(tickers: Vec<String>, output_path: &str) -> Result<()> {
    let rate_map = get_rate_map();
    let fmp_client = api::FMPClient::new(
        env::var("FINANCIALMODELINGPREP_API_KEY")
            .expect("FINANCIALMODELINGPREP_API_KEY must be set"),
    );
    let mut stocks = Vec::new();

    for ticker in tickers {
        if let Ok(details) = fmp_client.get_details(&ticker, &rate_map).await {
            stocks.push(models::Stock {
                ticker: details.ticker,
                name: details.name.unwrap_or_default(),
                market_cap: details.market_cap.unwrap_or_default(),
                currency_name: details.currency_name.unwrap_or_default(),
                currency_symbol: details.currency_symbol.unwrap_or_default(),
                active: details.active.unwrap_or_default(),
                description: details.description.unwrap_or_default(),
                homepage_url: details.homepage_url.unwrap_or_default(),
                employees: details.employees.unwrap_or_default(),
                revenue: details.revenue.unwrap_or_default(),
                revenue_usd: details.revenue_usd.unwrap_or_default(),
                working_capital_ratio: details.working_capital_ratio.unwrap_or_default(),
                quick_ratio: details.quick_ratio.unwrap_or_default(),
                eps: details.eps.unwrap_or_default(),
                pe_ratio: details.pe_ratio.unwrap_or_default(),
                debt_equity_ratio: details.debt_equity_ratio.unwrap_or_default(),
                roe: details.roe.unwrap_or_default(),
            });
        } else {
            eprintln!("Error fetching details for {}", ticker);
        }
    }

    let json = serde_json::to_string_pretty(&stocks)?;
    std::fs::write(output_path, json)?;
    println!("✅ JSON file created at: {}", output_path);
    Ok(())
}

fn generate_market_heatmap(results: &[(f64, Vec<String>)], output_path: &str) -> Result<()> {
    let stocks: Vec<viz::StockData> = results
        .iter()
        .map(|(market_cap, record)| {
            viz::StockData {
                symbol: record[0].clone(), // Ticker is at index 0
                market_cap_eur: *market_cap,
                employees: record[11].clone(), // Employees is at index 11
            }
        })
        .collect();

    viz::create_market_heatmap(stocks, output_path)
}

fn get_rate_map() -> HashMap<String, f64> {
    let mut rate_map = HashMap::new();

    // Base rates (currency to USD)
    rate_map.insert("EUR/USD".to_string(), 1.08);
    rate_map.insert("GBP/USD".to_string(), 1.25);
    rate_map.insert("CHF/USD".to_string(), 1.14);
    rate_map.insert("SEK/USD".to_string(), 0.096);
    rate_map.insert("DKK/USD".to_string(), 0.145);
    rate_map.insert("NOK/USD".to_string(), 0.093);
    rate_map.insert("JPY/USD".to_string(), 0.0068);
    rate_map.insert("HKD/USD".to_string(), 0.128);
    rate_map.insert("CNY/USD".to_string(), 0.139);
    rate_map.insert("BRL/USD".to_string(), 0.203);
    rate_map.insert("CAD/USD".to_string(), 0.737);
    rate_map.insert("ILS/USD".to_string(), 0.27); // Israeli Shekel rate
    rate_map.insert("ZAR/USD".to_string(), 0.053); // South African Rand rate

    // Add reverse rates (USD to currency)
    let mut pairs_to_add = Vec::new();
    for (pair, &rate) in rate_map.clone().iter() {
        if let Some((from, to)) = pair.split_once('/') {
            pairs_to_add.push((format!("{}/{}", to, from), 1.0 / rate));
        }
    }

    // Add cross rates (currency to currency)
    let base_pairs: Vec<_> = rate_map.clone().into_iter().collect();
    for (pair1, rate1) in &base_pairs {
        if let Some((from1, "USD")) = pair1.split_once('/') {
            for (pair2, rate2) in &base_pairs {
                if let Some(("USD", to2)) = pair2.split_once('/') {
                    if from1 != to2 {
                        // Calculate cross rate: from1/to2 = (from1/USD) * (USD/to2)
                        pairs_to_add.push((format!("{}/{}", from1, to2), rate1 * rate2));
                    }
                }
            }
        }
    }

    // Add all the new pairs
    for (pair, rate) in pairs_to_add {
        rate_map.insert(pair, rate);
    }

    // Debug print
    println!("Available rates:");
    for (pair, rate) in &rate_map {
        println!("{}: {}", pair, rate);
    }

    rate_map
}

/// Find the latest file in the output directory that matches a pattern
fn find_latest_file(pattern: &str) -> Result<PathBuf> {
    let paths: Vec<PathBuf> = glob(pattern)?.filter_map(|entry| entry.ok()).collect();

    let latest_file = paths
        .iter()
        .max_by_key(|path| path.metadata().unwrap().modified().unwrap())
        .ok_or_else(|| {
            anyhow::anyhow!("No files matching '{}' found in output directory", pattern)
        })?;

    Ok(latest_file.to_path_buf())
}

/// Read CSV file and return records with market cap in EUR
fn read_csv_with_market_cap(file_path: &Path) -> Result<Vec<(f64, Vec<String>)>> {
    let mut rdr = csv::Reader::from_path(file_path)?;
    let headers = rdr.headers()?.clone();
    let market_cap_idx = headers
        .iter()
        .position(|h| h == "Market Cap (USD)")
        .ok_or_else(|| anyhow::anyhow!("Market Cap (USD) column not found"))?;

    let mut results = Vec::new();

    for record in rdr.records() {
        let record = record?;
        if let Ok(market_cap) = record[market_cap_idx].parse::<f64>() {
            results.push((market_cap, record.iter().map(|s| s.to_string()).collect()));
        }
    }

    Ok(results)
}

/// Generate a heatmap from the latest top 100 active tickers CSV file
fn generate_heatmap_from_latest() -> Result<()> {
    let latest_file = find_latest_file("output/top_100_active_")?;
    println!("Reading from latest file: {:?}", latest_file);

    let results = read_csv_with_market_cap(&latest_file)?;

    // Generate the heatmap
    generate_market_heatmap(&results, "output/market_heatmap.png")?;
    println!("✅ Market heatmap generated from latest top 100 active tickers");

    Ok(())
}

/// Output the top 100 active tickers from the latest combined marketcaps CSV file
pub fn output_top_100_active() -> Result<()> {
    let latest_file = find_latest_file("output/combined_marketcaps_")?;
    println!("Reading from latest file: {:?}", latest_file);

    // Read and parse the CSV
    let mut rdr = csv::Reader::from_path(&latest_file)?;
    let headers = rdr.headers()?.clone();

    // Parse records and filter active ones
    let mut records: Vec<csv::StringRecord> = rdr
        .records()
        .filter_map(|record| record.ok())
        .filter(|record| {
            // Get the "Active" column (index 8) and check if it's "true"
            record
                .get(8)
                .map(|active| active == "true")
                .unwrap_or(false)
        })
        .collect();

    // Sort by market cap (EUR) in descending order
    records.sort_by(|a, b| {
        let a_cap: f64 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let b_cap: f64 = b.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        b_cap
            .partial_cmp(&a_cap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top 100
    let records = records.into_iter().take(100).collect::<Vec<_>>();

    // Create new CSV file for top 100
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let output_file = format!("output/top_100_active_{}.csv", timestamp);
    let mut writer = csv::Writer::from_path(&output_file)?;

    // Write headers and records
    writer.write_record(&headers)?;
    for record in records {
        writer.write_record(&record)?;
    }
    writer.flush()?;
    println!("✅ Top 100 active tickers written to: {}", output_file);

    // Generate heatmap from the new file
    let results = read_csv_with_market_cap(std::path::Path::new(&output_file))?;
    generate_market_heatmap(&results, "output/market_heatmap.png")?;
    println!("✅ Market heatmap generated");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&["top200-rs", "list-us"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::ListUs)));

        let cli = Cli::try_parse_from(&["top200-rs", "list-eu"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::ListEu)));

        let cli = Cli::try_parse_from(&["top200-rs", "export-us"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::ExportUs)));

        let cli = Cli::try_parse_from(&["top200-rs"]).unwrap();
        assert!(matches!(cli.command, None));
    }

    #[test]
    fn test_find_latest_file() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create output directory within temp dir
        let output_dir = dir_path.join("output");
        fs::create_dir(&output_dir).unwrap();

        // Create some test files
        fs::write(
            output_dir.join("combined_marketcaps_20240101_120000.csv"),
            "test",
        )
        .unwrap();
        fs::write(
            output_dir.join("combined_marketcaps_20240102_120000.csv"),
            "test",
        )
        .unwrap();
        let latest_file = output_dir.join("combined_marketcaps_20240103_120000.csv");
        fs::write(&latest_file, "test").unwrap();

        // Set current directory to temp dir for test
        std::env::set_current_dir(&dir_path).unwrap();

        let latest = find_latest_file("output/combined_marketcaps_*.csv").unwrap();
        assert_eq!(
            latest.canonicalize().unwrap(),
            latest_file.canonicalize().unwrap()
        );
    }

    #[test]
    fn test_get_rate_map() {
        let rate_map = get_rate_map();

        // Test that we have some rates
        assert!(!rate_map.is_empty());
        assert!(rate_map.len() > 0);

        // Test that we have at least one of the major currencies
        let has_major_currency = rate_map.contains_key("EUR/USD")
            || rate_map.contains_key("GBP/USD")
            || rate_map.contains_key("JPY/USD");
        assert!(
            has_major_currency,
            "Rate map should contain at least one major currency"
        );
    }

    #[tokio::test]
    async fn test_read_csv_with_market_cap() {
        let dir = tempdir().unwrap();
        let test_file = dir.path().join("test.csv");

        // Create a test CSV file with Market Cap column name matching the code
        let csv_content = "\
Ticker,Company Name,Market Cap (USD),Currency,Exchange,Price
AAPL,Apple Inc.,3000000000000,USD,NASDAQ,150.0
MSFT,Microsoft Corp.,2500000000000,USD,NASDAQ,300.0
GOOGL,Alphabet Inc.,2000000000000,USD,NASDAQ,130.0";

        fs::write(&test_file, csv_content).unwrap();

        let result = read_csv_with_market_cap(&test_file).unwrap();
        assert_eq!(result.len(), 3);

        // Check first entry
        let (market_cap, data) = &result[0];
        assert_relative_eq!(*market_cap, 3000000000000.0, epsilon = 0.01);
        assert_eq!(data[0], "AAPL");
    }

    #[test]
    fn test_convert_currency() {
        let rate_map = HashMap::from([
            ("EUR/USD".to_string(), 1.1),
            ("USD/EUR".to_string(), 0.91),
            ("GBP/USD".to_string(), 1.25),
            ("USD/GBP".to_string(), 0.8),
        ]);

        // Test USD to EUR conversion using relative comparison
        let result = convert_currency(100.0, "USD", "EUR", &rate_map);
        assert_relative_eq!(result, 91.0, epsilon = 0.01);

        // Test EUR to USD conversion
        let result = convert_currency(100.0, "EUR", "USD", &rate_map);
        assert_relative_eq!(result, 110.0, epsilon = 0.01);

        // Test same currency
        let result = convert_currency(100.0, "USD", "USD", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);
    }
}
