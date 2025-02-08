use crate::api::FMPClient;
use crate::currencies::insert_forex_rate;
use anyhow::Result;
use chrono::Local;
use sqlx::sqlite::SqlitePool;
use csv::Writer;
use std::path::PathBuf;

/// Update exchange rates in the database
pub async fn update_exchange_rates(fmp_client: &FMPClient, pool: &SqlitePool) -> Result<()> {
    // Fetch exchange rates
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

    // Store rates in database
    let timestamp = Local::now().timestamp();
    for rate in exchange_rates {
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            insert_forex_rate(pool, &name, price, price, timestamp).await?;
        }
    }

    println!("✅ Exchange rates updated in database");
    Ok(())
}

/// Export exchange rates to a CSV file
pub async fn export_exchange_rates_csv(fmp_client: &FMPClient, pool: &SqlitePool) -> Result<()> {
    // Fetch exchange rates
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

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("exchange_rates_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Name",
        "Price",
        "Changes Percentage",
        "Change",
        "Day Low",
        "Day High",
        "Year High",
        "Year Low",
        "Market Cap",
        "Price Avg 50",
        "Price Avg 200",
        "Volume",
        "Avg Volume",
        "Exchange",
        "Open",
        "Previous Close",
        "Timestamp",
    ])?;

    // Write data
    for rate in exchange_rates {
        writer.write_record(&[
            rate.name.unwrap_or_default(),
            rate.price.map(|p| p.to_string()).unwrap_or_default(),
            rate.changes_percentage.map(|cp| cp.to_string()).unwrap_or_default(),
            rate.change.map(|c| c.to_string()).unwrap_or_default(),
            rate.day_low.map(|dl| dl.to_string()).unwrap_or_default(),
            rate.day_high.map(|dh| dh.to_string()).unwrap_or_default(),
            rate.year_high.map(|yh| yh.to_string()).unwrap_or_default(),
            rate.year_low.map(|yl| yl.to_string()).unwrap_or_default(),
            rate.market_cap.map(|mc| mc.to_string()).unwrap_or_default(),
            rate.price_avg_50.map(|pa50| pa50.to_string()).unwrap_or_default(),
            rate.price_avg_200.map(|pa200| pa200.to_string()).unwrap_or_default(),
            rate.volume.map(|v| v.to_string()).unwrap_or_default(),
            rate.avg_volume.map(|av| av.to_string()).unwrap_or_default(),
            rate.exchange.unwrap_or_default(),
            rate.open.map(|o| o.to_string()).unwrap_or_default(),
            rate.previous_close.map(|pc| pc.to_string()).unwrap_or_default(),
            rate.timestamp.to_string(),
        ])?;
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", csv_path.display());

    Ok(())
}
