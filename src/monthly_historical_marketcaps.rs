// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api;
use crate::config;
use crate::currencies::{convert_currency_with_rate, get_rate_map_from_db_for_date};
use anyhow::Result;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

/// Fetches historical market caps for the last day of each month within the specified year range
pub async fn fetch_monthly_historical_marketcaps(
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
        "Fetching monthly historical market caps from {} to {}",
        start_year, end_year
    );

    for year in start_year..=end_year {
        for month in 1..=12 {
            // Skip future months in the current year
            if year == Utc::now().year() && month > Utc::now().month() {
                break;
            }

            // Get the last day of the month at 23:59
            let last_day = get_last_day_of_month(year, month);
            let time = NaiveTime::from_hms_opt(23, 59, 0).unwrap();
            let naive_dt = NaiveDateTime::new(last_day, time);
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
    }

    Ok(())
}

/// Helper function to get the last day of a given month
fn get_last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let first_day_next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    first_day_next_month.pred_opt().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_last_day_of_month() {
        assert_eq!(
            get_last_day_of_month(2025, 1),
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()
        );
        assert_eq!(
            get_last_day_of_month(2025, 2),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
        assert_eq!(
            get_last_day_of_month(2024, 2), // Leap year
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
        assert_eq!(
            get_last_day_of_month(2025, 4),
            NaiveDate::from_ymd_opt(2025, 4, 30).unwrap()
        );
        assert_eq!(
            get_last_day_of_month(2025, 12),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()
        );
    }
}
