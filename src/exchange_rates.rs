use crate::api::FMPClient;
use crate::db;
use anyhow::Result;
use chrono::Local;
use std::fs;

pub async fn export_exchange_rates_csv(fmp_client: &FMPClient) -> Result<()> {
    println!("Fetching current exchange rates...");

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/exchange_rates_{}.csv", timestamp);
    fs::create_dir_all("output")?;
    let file = fs::File::create(&filename)?;
    let mut writer = csv::Writer::from_writer(file);

    // Create database connection pool
    let db_url = "sqlite:top200.db";
    let pool = db::create_db_pool(db_url).await?;

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
            // Store rates in database
            db::store_forex_rates(&pool, &rates).await?;

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
                    &rate.price.map_or_else(|| "".to_string(), |v| v.to_string()),
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
            println!("✅ Exchange rates written to CSV and database");
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

    println!("\n✅ CSV file created at: {}", filename);
    Ok(())
}
