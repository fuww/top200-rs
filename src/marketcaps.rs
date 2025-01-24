use crate::api;
use crate::config;
use crate::currencies::{convert_currency, get_rate_map_from_db};
use crate::models;
use anyhow::Result;
use chrono::Local;
use csv::Writer;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

/// Store market cap data in the database
async fn store_market_cap(pool: &SqlitePool, details: &models::Details, rate_map: &std::collections::HashMap<String, f64>) -> Result<()> {
    let original_market_cap = details.market_cap.unwrap_or(0.0) as i64;
    let currency = details.currency_symbol.clone().unwrap_or_default();
    let eur_market_cap = convert_currency(original_market_cap as f64, &currency, "EUR", rate_map) as i64;
    let usd_market_cap = convert_currency(original_market_cap as f64, &currency, "USD", rate_map) as i64;
    let timestamp = Local::now().naive_utc().and_utc().timestamp();
    let name = details.name.as_ref().unwrap_or(&String::new()).to_string();
    let currency_name = details.currency_name.as_ref().unwrap_or(&String::new()).to_string();
    let description = details.description.as_ref().unwrap_or(&String::new()).to_string();
    let homepage_url = details.homepage_url.as_ref().unwrap_or(&String::new()).to_string();
    let active = details.active.unwrap_or(true);

    sqlx::query!(
        r#"
        INSERT INTO market_caps (
            ticker, name, market_cap_original, original_currency, market_cap_eur, market_cap_usd,
            exchange, active, description, homepage_url, timestamp
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        details.ticker,
        name,
        original_market_cap,
        currency,
        eur_market_cap,
        usd_market_cap,
        currency_name,
        active,
        description,
        homepage_url,
        timestamp,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetch market cap data from the database
async fn get_market_caps(pool: &SqlitePool) -> Result<Vec<(f64, Vec<String>)>> {
    let rows = sqlx::query!(
        r#"
        SELECT ticker, name, market_cap_original, original_currency, market_cap_eur, market_cap_usd,
               exchange, active, description, homepage_url, strftime('%s', timestamp) as timestamp
        FROM market_caps
        WHERE timestamp = (SELECT MAX(timestamp) FROM market_caps)
        "#
    )
    .fetch_all(pool)
    .await?;

    let results = rows.into_iter()
        .map(|row| {
            (
                row.market_cap_eur.unwrap_or(0) as f64,
                vec![
                    row.ticker.clone(), // Symbol
                    row.ticker,         // Ticker
                    row.name,
                    row.market_cap_original.unwrap_or(0).to_string(),
                    row.original_currency.unwrap_or_default(),
                    row.market_cap_eur.unwrap_or(0).to_string(),
                    row.market_cap_usd.unwrap_or(0).to_string(),
                    row.exchange.unwrap_or_default(),
                    row.active.unwrap_or(true).to_string(),
                    row.description.unwrap_or_default(),
                    row.homepage_url.unwrap_or_default(),
                    row.timestamp.map(|t| t.to_string()).unwrap_or_default(),
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
    for ticker in &tickers {
        let rate_map = rate_map.clone();
        let fmp_client = fmp_client.clone();

        if let Ok(details) = fmp_client.get_details(ticker, &rate_map).await {
            if let Err(e) = store_market_cap(pool, &details, &rate_map).await {
                eprintln!("Failed to store market cap for {}: {}", ticker, e);
            }
        }
        progress.inc(1);
    }
    progress.finish();
    println!("✅ Market cap data updated in database");

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
        "Timestamp",
    ])?;

    // Write data
    for (_, record) in &results {
        writer.write_record(record)?;
    }

    println!("✅ Market cap data exported to {}", filename);
    Ok(())
}

/// Main entry point for market cap functionality
pub async fn marketcaps(pool: &SqlitePool) -> Result<()> {
    // First update the database
    update_market_caps(pool).await?;
    
    // Then export the data
    export_market_caps(pool).await?;

    Ok(())
}
