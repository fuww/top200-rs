// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api;
use crate::config;
use crate::currencies::{convert_currency_with_rate, get_rate_map_from_db_for_date};
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

pub async fn fetch_historical_marketcaps(
    pool: &SqlitePool,
    start_year: i32,
    end_year: i32,
) -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

    // Get FMP client for market data
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = Arc::new(api::FMPClient::new(api_key));

    println!(
        "Fetching historical market caps from {} to {}",
        start_year, end_year
    );

    for year in start_year..=end_year {
        // Get Dec 31st of each year
        let date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
        let naive_dt = NaiveDateTime::new(date, NaiveTime::default());
        let datetime_utc = naive_dt.and_utc();
        let timestamp = naive_dt.and_utc().timestamp();
        println!("Fetching exchange rates for {}", naive_dt);
        let rate_map = get_rate_map_from_db_for_date(pool, Some(timestamp)).await?;

        for ticker in &tickers {
            match fmp_client
                .get_historical_market_cap(ticker, &datetime_utc)
                .await
            {
                Ok(market_cap) => {
                    // Convert currencies with rate information
                    let eur_result = convert_currency_with_rate(
                        market_cap.market_cap_original,
                        &market_cap.original_currency,
                        "EUR",
                        &rate_map,
                    );

                    let usd_result = convert_currency_with_rate(
                        market_cap.market_cap_original,
                        &market_cap.original_currency,
                        "USD",
                        &rate_map,
                    );

                    // Store the Unix timestamp of the historical date
                    let timestamp = naive_dt.and_utc().timestamp();

                    // Insert into database (use OR REPLACE to handle re-runs gracefully)
                    sqlx::query!(
                        r#"
                        INSERT OR REPLACE INTO market_caps (
                            ticker, name, market_cap_original, original_currency,
                            market_cap_eur, market_cap_usd, eur_rate, usd_rate,
                            exchange, price, active, timestamp
                        )
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                        ticker,
                        market_cap.name,
                        market_cap.market_cap_original,
                        market_cap.original_currency,
                        eur_result.amount,
                        usd_result.amount,
                        eur_result.rate,
                        usd_result.rate,
                        market_cap.exchange,
                        market_cap.price,
                        true,
                        timestamp,
                    )
                    .execute(pool)
                    .await?;

                    println!(
                        "✅ Added historical market cap for {} on {}",
                        ticker, naive_dt
                    );
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to fetch market cap for {} on {}: {}",
                        ticker, naive_dt, e
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_year_range_iteration() {
        // Test that year range iteration works correctly
        let start_year = 2020;
        let end_year = 2024;
        let years: Vec<i32> = (start_year..=end_year).collect();

        assert_eq!(years.len(), 5);
        assert_eq!(years[0], 2020);
        assert_eq!(years[4], 2024);
    }

    #[test]
    fn test_december_31st_date_creation() {
        // Test creating Dec 31st dates for various years
        for year in 2020..=2025 {
            let date = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
            assert_eq!(date.year(), year);
            assert_eq!(date.month(), 12);
            assert_eq!(date.day(), 31);
        }
    }

    #[test]
    fn test_naive_datetime_creation() {
        // Test creating NaiveDateTime for historical dates
        let date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let naive_dt = NaiveDateTime::new(date, NaiveTime::default());

        assert_eq!(naive_dt.year(), 2023);
        assert_eq!(naive_dt.month(), 12);
        assert_eq!(naive_dt.day(), 31);
        assert_eq!(naive_dt.hour(), 0);
        assert_eq!(naive_dt.minute(), 0);
        assert_eq!(naive_dt.second(), 0);
    }

    #[test]
    fn test_timestamp_conversion() {
        // Test that timestamps are created correctly
        let date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let naive_dt = NaiveDateTime::new(date, NaiveTime::default());
        let timestamp = naive_dt.and_utc().timestamp();

        // Dec 31, 2023 00:00:00 UTC should be around 1703980800
        assert!(timestamp > 0);
        assert!(timestamp > 1700000000); // After 2023-11-14
        assert!(timestamp < 1710000000); // Before 2024-03-09
    }

    #[test]
    fn test_year_range_boundaries() {
        // Test edge cases for year ranges
        let start_year = 2022;
        let end_year = 2022; // Same year
        let years: Vec<i32> = (start_year..=end_year).collect();

        assert_eq!(years.len(), 1);
        assert_eq!(years[0], 2022);
    }

    #[test]
    fn test_empty_year_range() {
        // Test invalid year range (start > end)
        let start_year = 2025;
        let end_year = 2020;
        let years: Vec<i32> = (start_year..=end_year).collect();

        assert!(years.is_empty());
    }

    #[test]
    fn test_utc_datetime_conversion() {
        // Test converting NaiveDateTime to UTC DateTime
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let naive_dt = NaiveDateTime::new(date, NaiveTime::default());
        let datetime_utc = naive_dt.and_utc();

        assert_eq!(datetime_utc.year(), 2024);
        assert_eq!(datetime_utc.month(), 6);
        assert_eq!(datetime_utc.day(), 15);
    }
}
