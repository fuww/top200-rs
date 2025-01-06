mod api;
mod models;
mod viz;
mod config;
mod utils;

use std::{collections::HashMap, env, path::PathBuf};
use anyhow::Result;
use chrono::{Local, NaiveDate};
use clap::{Parser, ValueEnum};
use csv::Writer;
use dotenv::dotenv;
use tokio;

pub use utils::convert_currency;

#[derive(Debug, Clone, ValueEnum, Default)]
enum Command {
    #[default]
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
    /// Generate Market Heatmap
    Heatmap,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Command to execute (defaults to export-combined)
    #[arg(value_enum, default_value_t = Command::ExportCombined)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let args = Args::parse();

    match args.command {
        Command::ExportCombined => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            export_details_combined_csv(&fmp_client).await?;
        }
        Command::ExportRates => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            export_exchange_rates_csv(&fmp_client).await?;
        }
        Command::ListUs => list_details_us().await?,
        Command::ListEu => list_details_eu().await?,
        Command::ExportUs => export_details_us_csv().await?,
        Command::ExportEu => export_details_eu_csv().await?,
        Command::Heatmap => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            export_details_combined_csv(&fmp_client).await?;
        }
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
                    &details.market_cap.map(|m| m.to_string()).unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.extra.get("exchange").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    &details.extra.get("price").map(|v| v.to_string()).unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details.revenue_usd.map(|r| r.to_string()).unwrap_or_default(),
                    &details.working_capital_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.quick_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.debt_equity_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    &ticker,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    &error_msg,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
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
        println!("\nFetching the marketcap for {} ({}/{}) ⌛️", ticker, i + 1, tickers.len());
        match client.get_details(ticker, date).await {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details.market_cap.map(|m| m.to_string()).unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details.revenue_usd.map(|r| r.to_string()).unwrap_or_default(),
                    &details.working_capital_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.quick_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.debt_equity_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    &ticker,
                    "",
                    "",
                    "",
                    "",
                    &error_msg,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
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
        println!("\nFetching the marketcap for {} ({}/{}) ⌛️", ticker, i + 1, tickers.len());
        match client.get_details(ticker, date).await {
            Ok(details) => {
                println!("Company: {}", details.name.unwrap_or_default());
                if let Some(market_cap) = details.market_cap {
                    println!("Market Cap: {} {}", details.currency_symbol.unwrap_or_default(), market_cap);
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
                    println!("Market Cap: {} {}", details.currency_symbol.unwrap_or_default(), market_cap);
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
        rate_map.insert(rate.name.clone(), rate.price);
    }

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("combined_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Ticker",
        "Company Name",
        "Market Cap (USD)",
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

    // Process tickers sequentially since we can't clone FMPClient
    let mut results = Vec::new();
    for ticker in tickers {
        match fmp_client.get_details(&ticker, &rate_map).await {
            Ok(details) => {
                let market_cap_usd = if let Some(market_cap) = details.market_cap {
                    if let Some(currency) = &details.currency_name {
                        utils::convert_currency(market_cap, currency, "USD", &rate_map)
                    } else {
                        market_cap // Assume USD if no currency specified
                    }
                } else {
                    0.0
                };

                if market_cap_usd > 0.0 {  // Only include companies with valid market cap
                    results.push((
                        market_cap_usd,
                        vec![
                            ticker.clone(),
                            details.name.unwrap_or_default(),
                            market_cap_usd.to_string(),
                            details.currency_name.unwrap_or_default(),
                            details.extra.get("exchange").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            details.extra.get("price").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                            details.active.map(|a| a.to_string()).unwrap_or_default(),
                            details.description.unwrap_or_default(),
                            details.homepage_url.unwrap_or_default(),
                            details.employees.map(|e| e.to_string()).unwrap_or_default(),
                            details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                            details.revenue_usd.map(|r| r.to_string()).unwrap_or_default(),
                            details.working_capital_ratio.map(|r| r.to_string()).unwrap_or_default(),
                            details.quick_ratio.map(|r| r.to_string()).unwrap_or_default(),
                            details.eps.map(|r| r.to_string()).unwrap_or_default(),
                            details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                            details.debt_equity_ratio.map(|r| r.to_string()).unwrap_or_default(),
                            details.roe.map(|r| r.to_string()).unwrap_or_default(),
                        ],
                    ));
                    println!("✅ Processed {}", ticker);
                } else {
                    eprintln!("Skipping {} - No valid market cap data", ticker);
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", ticker, e);
                // Continue processing other tickers
            }
        }
        
        // Add a small delay to stay within rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // Sort by market cap (highest to lowest)
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Write sorted results to CSV
    for (_, record) in &results {
        writer.write_record(&*record)?;
    }

    println!("📝 CSV file created: {}", csv_path.display());
    println!("💰 Results are sorted by market cap in USD (highest to lowest)");

    // Generate heatmap
    if let Err(e) = generate_market_heatmap(&results, "output/market_heatmap.svg") {
        eprintln!("Warning: Failed to generate heatmap: {}", e);
    } else {
        println!("📊 Market heatmap generated: output/market_heatmap.svg");
    }

    Ok(())
}

fn generate_market_heatmap(results: &[(f64, Vec<String>)], output_path: &str) -> Result<()> {
    let mut stocks = Vec::new();
    
    // Only take the first 100 results since they're already sorted by market cap
    for (market_cap, data) in results.iter().take(100) {
        if *market_cap > 0.0 {  // Skip error entries
            stocks.push((*market_cap, viz::StockData {
                symbol: data[0].clone(),
                market_cap_eur: *market_cap,
                employees: data[11].clone(),  
            }));
        }
    }

    println!("📊 Generating heatmap with top {} companies", stocks.len());
    viz::create_market_heatmap(stocks.into_iter().map(|(_, data)| data).collect(), output_path)?;
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
                let currencies: Vec<&str> = rate.name.split('/').collect();
                let (base, quote) = if currencies.len() == 2 {
                    (currencies[0], currencies[1])
                } else {
                    ("", "")
                };

                writer.write_record(&[
                    &rate.name,
                    &rate.price.to_string(),
                    &rate.changes_percentage.map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate.change.map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate.day_low.map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate.day_high.map_or_else(|| "".to_string(), |v| v.to_string()),
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
    let fmp_client = api::FMPClient::new(env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set"));

    for ticker in tickers {
        match fmp_client.get_details(&ticker, &rate_map).await {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details.market_cap.map(|m| m.to_string()).unwrap_or_default(),
                    &details.name.unwrap_or_default(),
                    &details.currency_name.unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                    &details.employees.unwrap_or_default(),
                    &details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                    &details.revenue_usd.map(|r| r.to_string()).unwrap_or_default(),
                    &details.working_capital_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.quick_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.eps.map(|r| r.to_string()).unwrap_or_default(),
                    &details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.debt_equity_ratio.map(|r| r.to_string()).unwrap_or_default(),
                    &details.roe.map(|r| r.to_string()).unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    &ticker,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    &error_msg,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", output_path);
    Ok(())
}

async fn export_marketcap_to_json(tickers: Vec<String>, output_path: &str) -> Result<()> {
    let rate_map = get_rate_map();
    let fmp_client = api::FMPClient::new(env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set"));
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
    rate_map.insert("ILS/USD".to_string(), 0.27);  // Adding Israeli Shekel rate
    
    rate_map
}
