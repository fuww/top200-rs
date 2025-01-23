// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api;
use crate::config;
use crate::currencies::{convert_currency, get_rate_map_from_db};
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
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

    println!("Fetching historical market caps from {} to {}", start_year, end_year);

    for year in start_year..=end_year {
        // Get Dec 31st of each year
        let timestamp = format!("{}-12-31 23:59:59", year);
        let naive_dt = NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S")?;
        let datetime_utc = DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);

        // Get exchange rates for that date
        println!("Fetching exchange rates for {}", timestamp);
        let rate_map = get_rate_map_from_db(pool).await?;

        for ticker in &tickers {
            match fmp_client.get_historical_market_cap(ticker, &datetime_utc).await {
                Ok(market_cap) => {
                    // Convert currencies if needed
                    let market_cap_eur = convert_currency(
                        market_cap.market_cap_original,
                        &market_cap.original_currency,
                        "EUR",
                        &rate_map,
                    );

                    let market_cap_usd = convert_currency(
                        market_cap.market_cap_original,
                        &market_cap.original_currency,
                        "USD",
                        &rate_map,
                    );

                    let timestamp_unix = naive_dt.and_utc().timestamp();

                    // Insert into database
                    sqlx::query!(
                        r#"
                        INSERT INTO market_caps (
                            ticker, name, market_cap_original, original_currency,
                            market_cap_eur, market_cap_usd, exchange, price,
                            active, timestamp
                        )
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                        ticker,
                        market_cap.name,
                        market_cap.market_cap_original,
                        market_cap.original_currency,
                        market_cap_eur,
                        market_cap_usd,
                        market_cap.exchange,
                        market_cap.price,
                        true,
                        timestamp_unix,
                    )
                    .execute(pool)
                    .await?;

                    println!("✅ Added historical market cap for {} on {}", ticker, timestamp);
                }
                Err(e) => {
                    eprintln!("❌ Failed to fetch market cap for {} on {}: {}", ticker, timestamp, e);
                }
            }
        }
    }

    Ok(())
}
