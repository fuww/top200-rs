use crate::api;
use crate::config;
use crate::currencies::{convert_currency, get_rate_map_from_db};
use anyhow::Result;
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

/// Fetches historical market caps for the last day of each month within the specified year range
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

    // Track last known market cap for each ticker
    let mut last_known_caps: HashMap<String, f64> = HashMap::new();

    for year in start_year..=end_year {
        for month in 1..=12 {
            // Skip future months in the current year
            let now = Utc::now();
            if year == now.year() && month > now.month() {
                break;
            }

            let datetime_utc = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(
                        year,
                        month,
                        get_last_day_of_month(year, month),
                    )
                    .unwrap(),
                    chrono::NaiveTime::from_hms_opt(23, 59, 0).unwrap(),
                ),
                Utc,
            );

            println!("Fetching exchange rates for {}", datetime_utc);
            let rate_map = get_rate_map_from_db(pool).await?;

            // Store timestamp to avoid temporary value issues
            let timestamp = datetime_utc.timestamp();

            for ticker in &tickers {
                // Check if we already have data for this date
                let existing = sqlx::query!(
                    "SELECT COUNT(*) as count FROM market_caps WHERE ticker = ? AND timestamp = ?",
                    ticker,
                    timestamp
                )
                .fetch_one(pool)
                .await?;

                if existing.count > 0 {
                    println!("Data already exists for {} on {}", ticker, datetime_utc);
                    continue;
                }

                match fmp_client
                    .get_historical_market_cap(ticker, &datetime_utc)
                    .await
                {
                    Ok(market_cap) => {
                        // Check if this is a repeated value
                        if let Some(last_cap) = last_known_caps.get(ticker) {
                            if *last_cap == market_cap.market_cap_original {
                                eprintln!(
                                    "Warning: Market cap for {} on {} is identical to previous value: {}",
                                    ticker,
                                    datetime_utc.format("%Y-%m-%d"),
                                    market_cap.market_cap_original
                                );
                            }
                        }

                        // Update last known cap
                        last_known_caps.insert(ticker.to_string(), market_cap.market_cap_original);

                        // Convert to EUR
                        let market_cap_eur = convert_currency(
                            market_cap.market_cap_original,
                            &market_cap.original_currency,
                            "EUR",
                            &rate_map,
                        );

                        // Store in database
                        sqlx::query!(
                            r#"
                            INSERT INTO market_caps (
                                ticker, 
                                name,
                                market_cap_original,
                                original_currency,
                                market_cap_eur,
                                exchange,
                                price,
                                timestamp
                            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                            "#,
                            market_cap.ticker,
                            market_cap.name,
                            market_cap.market_cap_original,
                            market_cap.original_currency,
                            market_cap_eur,
                            market_cap.exchange,
                            market_cap.price,
                            timestamp,
                        )
                        .execute(pool)
                        .await?;

                        println!("Added data for {} on {}", ticker, datetime_utc);
                    }
                    Err(e) => {
                        eprintln!("Error fetching market cap for {}: {}", ticker, e);
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_last_day_of_month(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_last_day_of_month() {
        assert_eq!(get_last_day_of_month(2024, 2), 29); // Leap year
        assert_eq!(get_last_day_of_month(2023, 2), 28); // Non-leap year
        assert_eq!(get_last_day_of_month(2024, 4), 30);
        assert_eq!(get_last_day_of_month(2024, 7), 31);
    }
}
