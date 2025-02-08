// SPDX-FileCopyrightText: 2025 Joost van der Laan
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::FMPClient;
use crate::currencies::insert_forex_rate;
use anyhow::Result;
use chrono::Local;
use sqlx::sqlite::SqlitePool;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ExchangeRate;
    use crate::db;
    use anyhow::anyhow;
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    // Mock FMP client for testing
    struct MockFMPClient {
        should_fail: bool,
        exchange_rates: Vec<ExchangeRate>,
    }

    impl MockFMPClient {
        fn new(should_fail: bool, exchange_rates: Vec<ExchangeRate>) -> Self {
            Self {
                should_fail,
                exchange_rates,
            }
        }

        async fn get_exchange_rates(&self) -> Result<Vec<ExchangeRate>> {
            if self.should_fail {
                return Err(anyhow!("API error"));
            }
            Ok(self.exchange_rates.clone())
        }
    }

    #[tokio::test]
    async fn test_successful_update() -> Result<()> {
        let pool = db::create_test_pool().await?;
        db::migrate_database(&pool).await?;

        // Create mock exchange rates
        let rates = vec![
            ExchangeRate {
                name: Some("EUR/USD".to_string()),
                price: Some(1.1),
                changes_percentage: None,
                change: None,
                day_low: None,
                day_high: None,
                year_high: None,
                year_low: None,
                market_cap: None,
                price_avg_50: None,
                price_avg_200: None,
                volume: None,
                avg_volume: None,
                exchange: None,
                open: None,
                previous_close: None,
                timestamp: 0,
            },
            ExchangeRate {
                name: Some("GBP/USD".to_string()),
                price: Some(1.3),
                changes_percentage: None,
                change: None,
                day_low: None,
                day_high: None,
                year_high: None,
                year_low: None,
                market_cap: None,
                price_avg_50: None,
                price_avg_200: None,
                volume: None,
                avg_volume: None,
                exchange: None,
                open: None,
                previous_close: None,
                timestamp: 0,
            },
        ];

        let client = MockFMPClient::new(false, rates);
        assert!(update_exchange_rates(&client, &pool).await.is_ok());

        // Verify rates were stored
        let stored_rates = sqlx::query_as::<_, (String, f64, f64)>(
            "SELECT symbol, ask, bid FROM forex_rates ORDER BY symbol",
        )
        .fetch_all(&pool)
        .await?;

        assert_eq!(stored_rates.len(), 2);
        assert_eq!(stored_rates[0].0, "EUR/USD");
        assert_eq!(stored_rates[0].1, 1.1);
        assert_eq!(stored_rates[1].0, "GBP/USD");
        assert_eq!(stored_rates[1].1, 1.3);

        Ok(())
    }

    #[tokio::test]
    async fn test_failed_api_call() -> Result<()> {
        let pool = db::create_test_pool().await?;
        db::migrate_database(&pool).await?;

        let client = MockFMPClient::new(true, vec![]);
        assert!(update_exchange_rates(&client, &pool).await.is_err());

        // Verify no rates were stored
        let stored_rates = sqlx::query_as::<_, (String, f64, f64)>(
            "SELECT symbol, ask, bid FROM forex_rates",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(stored_rates.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_exchange_rates() -> Result<()> {
        let pool = db::create_test_pool().await?;
        db::migrate_database(&pool).await?;

        // Create mock exchange rates with missing data
        let rates = vec![
            ExchangeRate {
                name: None, // Missing name
                price: Some(1.1),
                changes_percentage: None,
                change: None,
                day_low: None,
                day_high: None,
                year_high: None,
                year_low: None,
                market_cap: None,
                price_avg_50: None,
                price_avg_200: None,
                volume: None,
                avg_volume: None,
                exchange: None,
                open: None,
                previous_close: None,
                timestamp: 0,
            },
            ExchangeRate {
                name: Some("GBP/USD".to_string()),
                price: None, // Missing price
                changes_percentage: None,
                change: None,
                day_low: None,
                day_high: None,
                year_high: None,
                year_low: None,
                market_cap: None,
                price_avg_50: None,
                price_avg_200: None,
                volume: None,
                avg_volume: None,
                exchange: None,
                open: None,
                previous_close: None,
                timestamp: 0,
            },
        ];

        let client = MockFMPClient::new(false, rates);
        assert!(update_exchange_rates(&client, &pool).await.is_ok());

        // Verify no rates were stored since all were invalid
        let stored_rates = sqlx::query_as::<_, (String, f64, f64)>(
            "SELECT symbol, ask, bid FROM forex_rates",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(stored_rates.len(), 0);

        Ok(())
    }
}
