// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::FMPClient;
use crate::config;
use crate::currencies::{self, get_rate_map_from_db};
use anyhow::Result;
use chrono::{NaiveDate, NaiveTime};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn fetch_historical_market_caps(
    pool: &SqlitePool,
    start_year: i32,
    end_year: i32,
) -> Result<()> {
    let config = config::load_config()?;
    let mut all_tickers = config.us_tickers.clone();
    all_tickers.extend(config.non_us_tickers.clone());

    let mut ticker_count = HashMap::new();

    // Get existing records from database
    let existing_records = sqlx::query!(
        "SELECT ticker, strftime('%Y-%m-%d', datetime(timestamp, 'unixepoch')) as date
         FROM market_caps"
    )
    .fetch_all(pool)
    .await?;

    // Create a map of ticker -> set of dates for quick lookup
    let mut existing_dates: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for record in existing_records {
        existing_dates
            .entry(record.ticker)
            .or_insert_with(std::collections::HashSet::new)
            .insert(record.date.unwrap_or_default());
    }

    // Get FMP client for market data
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = Arc::new(FMPClient::new(api_key));

    for year in start_year..=end_year {
        for ticker in &all_tickers {
            let mut count = 0;

            // Get Dec 31st of each year
            let naive_dt = NaiveDate::from_ymd_opt(year, 12, 31)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
            let datetime_utc = naive_dt.and_utc();

            // Get exchange rates
            let rate_map = get_rate_map_from_db(pool).await?;

            match fmp_client.get_historical_market_cap(ticker, &datetime_utc).await {
                Ok(market_cap) => {
                    let date_str = naive_dt.format("%Y-%m-%d").to_string();

                    // Skip if we already have this date for this ticker
                    if let Some(dates) = existing_dates.get(ticker) {
                        if dates.contains(&date_str) {
                            continue;
                        }
                    }

                    // Convert currencies if needed
                    let market_cap_eur = currencies::convert_currency(
                        market_cap.market_cap_original,
                        &market_cap.original_currency,
                        "EUR",
                        &rate_map,
                    );

                    let market_cap_usd = currencies::convert_currency(
                        market_cap.market_cap_original,
                        &market_cap.original_currency,
                        "USD",
                        &rate_map,
                    );

                    let timestamp = naive_dt.and_utc().timestamp();

                    sqlx::query!(
                        "INSERT INTO market_caps (
                            ticker,
                            name,
                            market_cap_original,
                            original_currency,
                            market_cap_eur,
                            market_cap_usd,
                            exchange,
                            price,
                            active,
                            description,
                            homepage_url,
                            employees,
                            revenue,
                            revenue_usd,
                            working_capital_ratio,
                            quick_ratio,
                            eps,
                            pe_ratio,
                            de_ratio,
                            roe,
                            timestamp
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        ticker,
                        market_cap.name,
                        market_cap.market_cap_original,
                        market_cap.original_currency,
                        market_cap_eur,
                        market_cap_usd,
                        market_cap.exchange,
                        market_cap.price,
                        true,
                        None::<String>,  // description
                        None::<String>,  // homepage_url
                        None::<i32>,     // employees
                        None::<f64>,     // revenue
                        None::<f64>,     // revenue_usd
                        None::<f64>,     // working_capital_ratio
                        None::<f64>,     // quick_ratio
                        None::<f64>,     // eps
                        None::<f64>,     // pe_ratio
                        None::<f64>,     // de_ratio
                        None::<f64>,     // roe
                        timestamp
                    )
                    .execute(pool)
                    .await?;

                    count += 1;
                    println!("✅ Added historical market cap for {} on {}", ticker, naive_dt);
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to fetch market cap for {} on {}: {}",
                        ticker, naive_dt, e
                    );
                }
            }

            *ticker_count.entry(ticker.clone()).or_insert(0) += count;
        }
    }

    println!("\nSummary of records added:");
    for (ticker, count) in ticker_count {
        println!("{}: {}", ticker, count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fetch_historical_market_caps() -> Result<()> {
        // Skip test if API key is not set
        if std::env::var("FINANCIALMODELINGPREP_API_KEY").is_err() {
            println!("Skipping test as FINANCIALMODELINGPREP_API_KEY is not set");
            return Ok(());
        }

        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

        // Create SQLite database file
        std::fs::File::create(&db_path)?;

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS market_caps (
                ticker TEXT NOT NULL,
                name TEXT,
                market_cap_original REAL,
                original_currency TEXT,
                market_cap_eur REAL,
                market_cap_usd REAL,
                exchange TEXT,
                price REAL,
                active BOOLEAN,
                description TEXT,
                homepage_url TEXT,
                employees INTEGER,
                revenue REAL,
                revenue_usd REAL,
                working_capital_ratio REAL,
                quick_ratio REAL,
                eps REAL,
                pe_ratio REAL,
                de_ratio REAL,
                roe REAL,
                timestamp INTEGER NOT NULL,
                UNIQUE(ticker, timestamp)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Test with a small date range
        fetch_historical_market_caps(&pool, 2023, 2023).await?;

        // Verify that we have some data
        let count: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM market_caps")
            .fetch_one(&pool)
            .await?;

        assert!(count.0 > 0, "No records were inserted");

        Ok(())
    }
}
