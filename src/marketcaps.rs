use crate::api;
use crate::config;
use crate::currencies::{convert_currency, get_rate_map_from_db, update_currencies};
use crate::exchange_rates;
use crate::models;
use crate::ticker_details::{self, TickerDetails};
use anyhow::Result;
use chrono::Local;
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

/// Store market cap data in the database
async fn store_market_cap(pool: &SqlitePool, details: &models::Details, rate_map: &std::collections::HashMap<String, f64>, timestamp: i64) -> Result<()> {
    let original_market_cap = details.market_cap.unwrap_or(0.0) as i64;
    let currency = details.currency_symbol.clone().unwrap_or_default();
    let eur_market_cap = convert_currency(original_market_cap as f64, &currency, "EUR", rate_map) as i64;
    let usd_market_cap = convert_currency(original_market_cap as f64, &currency, "USD", rate_map) as i64;
    let name = details.name.as_ref().unwrap_or(&String::new()).to_string();
    let currency_name = details.currency_name.as_ref().unwrap_or(&String::new()).to_string();
    let active = details.active.unwrap_or(true);

    // Store market cap data
    sqlx::query!(
        r#"
        INSERT INTO market_caps (
            ticker, name, market_cap_original, original_currency, market_cap_eur, market_cap_usd,
            exchange, active, timestamp
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        details.ticker,
        name,
        original_market_cap,
        currency,
        eur_market_cap,
        usd_market_cap,
        currency_name,
        active,
        timestamp,
    )
    .execute(pool)
    .await?;

    // Store ticker details
    let ticker_details = TickerDetails {
        ticker: details.ticker.clone(),
        description: details.description.clone(),
        homepage_url: details.homepage_url.clone(),
        employees: details.employees.clone(),
    };
    ticker_details::update_ticker_details(pool, &ticker_details).await?;

    Ok(())
}

/// Fetch market cap data from the database
async fn get_market_caps(pool: &SqlitePool) -> Result<Vec<(f64, Vec<String>)>> {
    let records = sqlx::query!(
        r#"
        SELECT 
            m.ticker,
            m.name,
            m.market_cap_original,
            m.original_currency,
            m.market_cap_eur,
            m.market_cap_usd,
            m.exchange,
            m.active,
            strftime('%s', m.timestamp) as timestamp,
            td.description,
            td.homepage_url,
            td.employees
        FROM market_caps m
        LEFT JOIN ticker_details td ON m.ticker = td.ticker
        WHERE m.timestamp = (SELECT MAX(timestamp) FROM market_caps)
        "#
    )
    .fetch_all(pool)
    .await?;

    let results = records
        .into_iter()
        .map(|r| {
            let market_cap_eur = r.market_cap_eur.unwrap_or(0) as f64;
            (
                market_cap_eur,
                vec![
                    r.ticker.clone(),
                    r.ticker,
                    r.name,
                    r.market_cap_original.unwrap_or(0).to_string(),
                    r.original_currency.unwrap_or_default(),
                    r.market_cap_eur.unwrap_or(0).to_string(),
                    r.market_cap_usd.unwrap_or(0).to_string(),
                    r.exchange.unwrap_or_default(),
                    if r.active.unwrap_or(true) { "true".to_string() } else { "false".to_string() },
                    r.description.unwrap_or_default(),
                    r.homepage_url.unwrap_or_default(),
                    r.employees.map(|e| e.to_string()).unwrap_or_default(),
                    r.timestamp.unwrap_or_default().to_string(),
                ],
            )
        })
        .collect();

    Ok(results)
}

/// Update market cap data in the database
async fn update_market_caps(pool: &SqlitePool) -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

    // Get latest exchange rates from database
    println!("Fetching current exchange rates from database...");
    let rate_map = get_rate_map_from_db(pool).await?;
    println!("✅ Exchange rates fetched from database");

    // Get FMP client for market data
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = Arc::new(api::FMPClient::new(api_key));

    // Create a rate_map Arc for sharing between tasks
    let rate_map = Arc::new(rate_map);
    let total_tickers = tickers.len();

    // Use a single timestamp for all records
    let timestamp = Local::now().naive_utc().and_utc().timestamp();

    // Process tickers with progress tracking
    let progress = ProgressBar::new(total_tickers as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Update market cap data in database
    println!("Updating market cap data in database...");
    let mut failed_tickers = Vec::new();
    for ticker in &tickers {
        let rate_map = rate_map.clone();
        let fmp_client = fmp_client.clone();

        match fmp_client.get_details(ticker, &rate_map).await {
            Ok(details) => {
                if let Err(e) = store_market_cap(pool, &details, &rate_map, timestamp).await {
                    eprintln!("Failed to store market cap for {}: {}", ticker, e);
                    failed_tickers.push((ticker, format!("Failed to store market cap: {}", e)));
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch details for {}: {}", ticker, e);
                failed_tickers.push((ticker, format!("Failed to fetch details: {}", e)));
            }
        }
        progress.inc(1);
    }
    progress.finish();
    
    // Print summary of failed tickers
    if !failed_tickers.is_empty() {
        println!("\nFailed to process {} tickers:", failed_tickers.len());
        for (ticker, error) in &failed_tickers {
            println!("  {} - {}", ticker, error);
        }
    }
    
    println!("✅ Market cap data updated in database ({} successful, {} failed)",
             total_tickers - failed_tickers.len(),
             failed_tickers.len());

    Ok(())
}

/// Export market cap data to CSV
pub async fn export_market_caps(pool: &SqlitePool) -> Result<()> {
    // Get market cap data from database
    println!("Fetching market cap data from database...");
    let mut results = get_market_caps(pool).await?;
    println!("✅ Market cap data fetched from database");

    // Sort by EUR market cap
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Export to CSV
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/combined_marketcaps_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Symbol",
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Timestamp",
    ])?;

    // Write data
    for (_, record) in &results {
        writer.write_record(record)?;
    }

    println!("✅ Market cap data exported to {}", filename);
    Ok(())
}

/// Export top 100 active companies to CSV
pub async fn export_top_100_active(pool: &SqlitePool) -> Result<()> {
    // Get market cap data from database
    let mut results = get_market_caps(pool).await?;

    // Sort by EUR market cap
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Filter for active companies first, then take top 100
    let active_results: Vec<_> = results
        .iter()
        .filter(|(_, record)| record[8] == "true") // Active column
        .take(100)
        .collect();

    // Export to CSV
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/top_100_active_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Symbol",
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Timestamp",
    ])?;

    // Write data
    for (_, record) in active_results {
        writer.write_record(record)?;
    }

    println!("✅ Top 100 active companies exported to {}", filename);
    Ok(())
}

/// Main entry point for market cap functionality
pub async fn marketcaps(pool: &SqlitePool) -> Result<()> {
    // First update currencies and exchange rates
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = api::FMPClient::new(api_key);
    
    println!("Updating currencies and exchange rates...");
    update_currencies(&fmp_client, pool).await?;
    exchange_rates::update_exchange_rates(&fmp_client, pool).await?;
    
    // Then update market caps
    update_market_caps(pool).await?;
    
    // Export both the full list and top 100 active
    export_market_caps(pool).await?;
    export_top_100_active(pool).await?;

    Ok(())
}
