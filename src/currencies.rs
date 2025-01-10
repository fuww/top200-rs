// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Sqlite};
use std::collections::HashMap;

/// Insert a currency into the database
pub async fn insert_currency(pool: &SqlitePool, code: &str, name: &str) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO currencies (code, name)
        VALUES (?, ?)
        ON CONFLICT(code) DO UPDATE SET
            name = excluded.name,
            updated_at = CURRENT_TIMESTAMP
        "#,
        code,
        name
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a currency from the database by its code
pub async fn get_currency(pool: &SqlitePool, code: &str) -> Result<Option<(String, String)>> {
    let record = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT code, name
        FROM currencies
        WHERE code = ?
        "#,
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// List all currencies in the database
pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    let records = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT code, name
        FROM currencies
        ORDER BY code
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(records)
}

/// Get a map of exchange rates between currencies
pub fn get_rate_map() -> HashMap<String, f64> {
    let mut rate_map = HashMap::new();

    // Add EUR/USD rate
    rate_map.insert("EUR/USD".to_string(), 1.1);
    // Add EUR/GBP rate
    rate_map.insert("EUR/GBP".to_string(), 0.85);
    // Add EUR/JPY rate
    rate_map.insert("EUR/JPY".to_string(), 160.0);

    rate_map
}

/// Convert an amount from one currency to another using the rate map
pub fn convert_currency(
    amount: f64,
    from_currency: &str,
    to_currency: &str,
    rate_map: &HashMap<String, f64>,
) -> f64 {
    // If currencies are the same, return original amount
    if from_currency == to_currency {
        return amount;
    }

    // Try direct conversion
    let rate_key = format!("{}/{}", from_currency, to_currency);
    if let Some(&rate) = rate_map.get(&rate_key) {
        return amount * rate;
    }

    // Try inverse conversion
    let inverse_key = format!("{}/{}", to_currency, from_currency);
    if let Some(&rate) = rate_map.get(&inverse_key) {
        return amount / rate;
    }

    // If no conversion found, return original amount
    amount
}

/// Insert a forex rate into the database
pub async fn insert_forex_rate(
    pool: &SqlitePool,
    symbol: &str,
    ask: f64,
    bid: f64,
    timestamp: i64,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO forex_rates (symbol, ask, bid, timestamp)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(symbol, timestamp) DO UPDATE SET
            ask = excluded.ask,
            bid = excluded.bid,
            updated_at = CURRENT_TIMESTAMP
        "#,
        symbol,
        ask,
        bid,
        timestamp
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get the latest forex rate for a symbol
pub async fn get_latest_forex_rate(
    pool: &SqlitePool,
    symbol: &str,
) -> Result<Option<(f64, f64, i64)>> {
    let record = sqlx::query!(
        r#"
        SELECT ask, bid, timestamp
        FROM forex_rates
        WHERE symbol = ?
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
        symbol
    )
    .fetch_optional(pool)
    .await?;

    Ok(record.map(|r| (r.ask, r.bid, r.timestamp)))
}

/// Get all forex rates for a symbol within a time range
pub async fn get_forex_rates(
    pool: &SqlitePool,
    symbol: &str,
    from_timestamp: i64,
    to_timestamp: i64,
) -> Result<Vec<(f64, f64, i64)>> {
    let records = sqlx::query!(
        r#"
        SELECT ask, bid, timestamp
        FROM forex_rates
        WHERE symbol = ?
        AND timestamp BETWEEN ? AND ?
        ORDER BY timestamp ASC
        "#,
        symbol,
        from_timestamp,
        to_timestamp
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().map(|r| (r.ask, r.bid, r.timestamp)).collect())
}

/// List all unique symbols in the forex_rates table
pub async fn list_forex_symbols(pool: &SqlitePool) -> Result<Vec<String>> {
    let records = sqlx::query!(
        r#"
        SELECT DISTINCT symbol
        FROM forex_rates
        ORDER BY symbol
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().map(|r| r.symbol).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use approx::assert_relative_eq;

    #[tokio::test]
    async fn test_db_schema() -> Result<()> {
        // Set up database connection
        let db_url = "sqlite::memory:";
        let pool = crate::db::create_db_pool(db_url).await?;
        sqlx::migrate!().run(&pool).await?;

        // Test that we can insert and retrieve forex rates
        insert_forex_rate(&pool, "EURUSD", 1.07833, 1.07832, 1701956301).await?;

        // Check that we can retrieve the rate
        let rate = get_latest_forex_rate(&pool, "EURUSD").await?;
        assert!(rate.is_some());
        let (ask, bid, timestamp) = rate.unwrap();
        assert_relative_eq!(ask, 1.07833, epsilon = 0.00001);
        assert_relative_eq!(bid, 1.07832, epsilon = 0.00001);
        assert_eq!(timestamp, 1701956301);

        Ok(())
    }

    #[tokio::test]
    async fn test_currencies_in_database() -> Result<()> {
        // Set up database connection
        let db_url = "sqlite::memory:"; // Use in-memory database for testing
        let pool = db::create_db_pool(db_url).await?;
        sqlx::migrate!().run(&pool).await?;

        // Add all currencies to the database
        let currencies_data = [
            ("USD", "US Dollar"),
            ("EUR", "Euro"),
            ("GBP", "British Pound"),
            ("CHF", "Swiss Franc"),
            ("SEK", "Swedish Krona"),
            ("DKK", "Danish Krone"),
            ("NOK", "Norwegian Krone"),
            ("JPY", "Japanese Yen"),
            ("HKD", "Hong Kong Dollar"),
            ("CNY", "Chinese Yuan"),
            ("BRL", "Brazilian Real"),
            ("CAD", "Canadian Dollar"),
            ("ILS", "Israeli Shekel"),
            ("ZAR", "South African Rand"),
        ];

        for (code, name) in currencies_data {
            insert_currency(&pool, code, name).await?;
        }

        // Test listing all currencies
        let currencies = list_currencies(&pool).await?;
        assert_eq!(currencies.len(), currencies_data.len());

        // Test getting a specific currency
        let euro = get_currency(&pool, "EUR").await?;
        assert!(euro.is_some());
        let (code, name) = euro.unwrap();
        assert_eq!(code, "EUR");
        assert_eq!(name, "Euro");

        // Test updating a currency
        insert_currency(&pool, "EUR", "European Euro").await?;
        let euro = get_currency(&pool, "EUR").await?;
        assert!(euro.is_some());
        let (code, name) = euro.unwrap();
        assert_eq!(code, "EUR");
        assert_eq!(name, "European Euro");

        Ok(())
    }

    #[test]
    fn test_convert_currency() {
        let rate_map = get_rate_map();

        // Test EUR to USD conversion
        let result = convert_currency(100.0, "EUR", "USD", &rate_map);
        assert_relative_eq!(result, 110.0, epsilon = 0.01);

        // Test USD to EUR conversion
        let result = convert_currency(110.0, "USD", "EUR", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);

        // Test EUR to GBP conversion
        let result = convert_currency(100.0, "EUR", "GBP", &rate_map);
        assert_relative_eq!(result, 85.0, epsilon = 0.01);

        // Test GBP to EUR conversion
        let result = convert_currency(85.0, "GBP", "EUR", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);

        // Test EUR to JPY conversion
        let result = convert_currency(100.0, "EUR", "JPY", &rate_map);
        assert_relative_eq!(result, 16000.0, epsilon = 0.01);

        // Test JPY to EUR conversion
        let result = convert_currency(16000.0, "JPY", "EUR", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);

        // Test same currency
        let result = convert_currency(100.0, "USD", "USD", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);

        // Test missing rate
        let result = convert_currency(100.0, "XXX", "USD", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01); // Should return original amount
    }

    #[tokio::test]
    async fn test_forex_rates() -> Result<()> {
        // Set up database connection
        let db_url = "sqlite::memory:";
        let pool = crate::db::create_db_pool(db_url).await?;
        sqlx::migrate!().run(&pool).await?;

        // Insert some test data
        insert_forex_rate(&pool, "EURUSD", 1.07833, 1.07832, 1701956301).await?;
        insert_forex_rate(&pool, "EURUSD", 1.07834, 1.07833, 1701956302).await?;
        insert_forex_rate(&pool, "GBPUSD", 1.25001, 1.25000, 1701956301).await?;

        // Test getting latest rate
        let latest = get_latest_forex_rate(&pool, "EURUSD").await?;
        assert!(latest.is_some());
        let (ask, bid, timestamp) = latest.unwrap();
        assert_relative_eq!(ask, 1.07834, epsilon = 0.00001);
        assert_relative_eq!(bid, 1.07833, epsilon = 0.00001);
        assert_eq!(timestamp, 1701956302);

        // Test getting rates in range
        let rates = get_forex_rates(&pool, "EURUSD", 1701956300, 1701956303).await?;
        assert_eq!(rates.len(), 2);

        // Test listing symbols
        let symbols = list_forex_symbols(&pool).await?;
        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains(&"EURUSD".to_string()));
        assert!(symbols.contains(&"GBPUSD".to_string()));

        // Test getting non-existent rate
        let missing = get_latest_forex_rate(&pool, "XXXYYY").await?;
        assert!(missing.is_none());

        // Test getting rates with empty range
        let empty_range = get_forex_rates(&pool, "EURUSD", 1701956303, 1701956304).await?;
        assert!(empty_range.is_empty());

        // Test rate update with same timestamp (should update values)
        insert_forex_rate(&pool, "EURUSD", 1.07835, 1.07834, 1701956302).await?;
        let updated = get_latest_forex_rate(&pool, "EURUSD").await?;
        assert!(updated.is_some());
        let (ask, bid, timestamp) = updated.unwrap();
        assert_relative_eq!(ask, 1.07835, epsilon = 0.00001);
        assert_relative_eq!(bid, 1.07834, epsilon = 0.00001);
        assert_eq!(timestamp, 1701956302);

        Ok(())
    }

    #[tokio::test]
    async fn test_currency_operations() -> Result<()> {
        let db_url = "sqlite::memory:";
        let pool = crate::db::create_db_pool(db_url).await?;
        sqlx::migrate!().run(&pool).await?;

        // Test inserting and retrieving a currency
        insert_currency(&pool, "EUR", "Euro").await?;
        let euro = get_currency(&pool, "EUR").await?;
        assert!(euro.is_some());
        let (code, name) = euro.unwrap();
        assert_eq!(code, "EUR");
        assert_eq!(name, "Euro");

        // Test updating a currency
        insert_currency(&pool, "EUR", "European Euro").await?;
        let euro = get_currency(&pool, "EUR").await?;
        assert!(euro.is_some());
        let (code, name) = euro.unwrap();
        assert_eq!(code, "EUR");
        assert_eq!(name, "European Euro");

        Ok(())
    }
}
