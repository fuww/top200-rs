// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

/// Get a map of exchange rates between currencies from the database
pub async fn get_rate_map_from_db(pool: &SqlitePool) -> Result<HashMap<String, f64>> {
    let rates = sqlx::query!(
        "SELECT symbol, ask FROM forex_rates ORDER BY timestamp DESC"
    )
    .fetch_all(pool)
    .await?;

    let mut rate_map = HashMap::new();
    for rate in rates {
        let key = rate.symbol;
        rate_map.insert(key, rate.ask);
    }

    Ok(rate_map)
}

/// Convert an amount from one currency to another using the rate map
pub fn convert_currency(
    amount: f64,
    from_currency: &str,
    to_currency: &str,
    rate_map: &HashMap<String, f64>,
) -> f64 {
    if from_currency == to_currency {
        return amount;
    }

    let key = format!("{}/{}", from_currency, to_currency);
    if let Some(&rate) = rate_map.get(&key) {
        return amount * rate;
    }

    // Try reverse conversion
    let reverse_key = format!("{}/{}", to_currency, from_currency);
    if let Some(&rate) = rate_map.get(&reverse_key) {
        return amount / rate;
    }

    // If no direct conversion found, try through USD
    if from_currency != "USD" && to_currency != "USD" {
        let usd_amount = convert_currency(amount, from_currency, "USD", rate_map);
        return convert_currency(usd_amount, "USD", to_currency, rate_map);
    }

    amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::create_db_pool;
    use tempfile::tempdir;

    fn assert_float_eq(a: f64, b: f64) {
        let epsilon = 1e-10;
        assert!((a - b).abs() < epsilon, "Expected {} to be equal to {}", a, b);
    }

    #[tokio::test]
    async fn test_db_schema() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

        let pool = create_db_pool(&db_url).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS forex_rates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                ask REAL NOT NULL,
                bid REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                UNIQUE(symbol, timestamp)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_convert_currency() -> Result<()> {
        let mut rate_map = HashMap::new();
        rate_map.insert("EUR/USD".to_string(), 1.1);
        rate_map.insert("GBP/USD".to_string(), 1.3);

        // Direct conversion
        assert_float_eq(convert_currency(100.0, "EUR", "USD", &rate_map), 110.0);

        // Reverse conversion
        assert_float_eq(convert_currency(110.0, "USD", "EUR", &rate_map), 100.0);

        // Through USD
        assert_float_eq(convert_currency(100.0, "EUR", "GBP", &rate_map), 84.61538461538461);

        // Same currency
        assert_float_eq(convert_currency(100.0, "USD", "USD", &rate_map), 100.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_rate_map_from_db() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

        let pool = create_db_pool(&db_url).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS forex_rates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                ask REAL NOT NULL,
                bid REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                UNIQUE(symbol, timestamp)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query!(
            "INSERT INTO forex_rates (symbol, ask, bid, timestamp) VALUES (?, ?, ?, ?)",
            "EUR/USD",
            1.1,
            1.09,
            1704067200
        )
        .execute(&pool)
        .await?;

        let rate_map = get_rate_map_from_db(&pool).await?;
        assert_eq!(rate_map.get("EUR/USD"), Some(&1.1));

        Ok(())
    }
}
