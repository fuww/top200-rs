// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::FMPClient;
use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

/// Insert a currency into the database
pub async fn insert_currency(pool: &SqlitePool, code: &str, name: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO currencies (code, name)
        VALUES (?, ?)
        ON CONFLICT(code) DO UPDATE SET
            name = excluded.name,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(code)
    .bind(name)
    .execute(pool)
    .await?;

    Ok(())
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

/// Get a map of exchange rates between currencies from the database
pub async fn get_rate_map_from_db(pool: &SqlitePool) -> Result<HashMap<String, f64>> {
    let mut rate_map = HashMap::new();

    // Get all unique symbols from the database
    let symbols = list_forex_symbols(pool).await?;

    // Get latest rates for each symbol
    for symbol in symbols {
        if let Some((ask, _bid, _timestamp)) = get_latest_forex_rate(pool, &symbol).await? {
            let (from, to) = symbol.split_once('/').unwrap();
            rate_map.insert(format!("{}/{}", from, to), ask);
            rate_map.insert(format!("{}/{}", to, from), 1.0 / ask);
        }
    }

    // Add cross rates
    let pairs: Vec<_> = rate_map.clone().into_iter().collect();
    for (pair1, rate1) in &pairs {
        if let Some((from1, to1)) = pair1.split_once('/') {
            for (pair2, rate2) in &pairs {
                if let Some((from2, to2)) = pair2.split_once('/') {
                    if to1 == from2 && from1 != to2 {
                        let cross_pair = format!("{}/{}", from1, to2);
                        if !rate_map.contains_key(&cross_pair) {
                            rate_map.insert(cross_pair.clone(), rate1 * rate2);
                            rate_map.insert(format!("{}/{}", to2, from1), 1.0 / (rate1 * rate2));
                        }
                    }
                }
            }
        }
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

    // Handle special cases for currency subunits and alternative codes
    let (adjusted_amount, adjusted_from_currency) = match from_currency {
        "GBp" => (amount / 100.0, "GBP"), // Convert pence to pounds
        "ZAc" => (amount / 100.0, "ZAR"),
        "ILA" => (amount, "ILS"),
        _ => (amount, from_currency),
    };

    // Adjust target currency if needed
    let adjusted_to_currency = match to_currency {
        "GBp" => "GBP", // Also handle GBp as target currency
        "ZAc" => "ZAR", // Also handle ZAc as target currency
        "ILA" => "ILS",
        _ => to_currency,
    };

    // Try direct conversion first
    let direct_rate = format!("{}/{}", adjusted_from_currency, adjusted_to_currency);
    if let Some(&rate) = rate_map.get(&direct_rate) {
        let result = adjusted_amount * rate;
        return match to_currency {
            "GBp" => result * 100.0,
            "ZAc" => result * 100.0,
            _ => result,
        };
    }

    // Try reverse rate
    let reverse_rate = format!("{}/{}", adjusted_to_currency, adjusted_from_currency);
    if let Some(&rate) = rate_map.get(&reverse_rate) {
        let result = adjusted_amount * (1.0 / rate);
        return match to_currency {
            "GBp" => result * 100.0,
            "ZAc" => result * 100.0,
            _ => result,
        };
    }

    // Try conversion through intermediate currencies
    for (pair, &rate1) in rate_map {
        if let Some((from1, to1)) = pair.split_once('/') {
            if from1 == adjusted_from_currency {
                let second_leg = format!("{}/{}", to1, adjusted_to_currency);
                if let Some(&rate2) = rate_map.get(&second_leg) {
                    let result = adjusted_amount * rate1 * rate2;
                    return match to_currency {
                        "GBp" => result * 100.0,
                        "ZAc" => result * 100.0,
                        _ => result,
                    };
                }
            }
        }
    }

    // If no conversion rate is found, return the original amount
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
    sqlx::query(
        r#"
        INSERT INTO forex_rates (symbol, ask, bid, timestamp)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(symbol, timestamp) DO UPDATE SET
            ask = excluded.ask,
            bid = excluded.bid,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(symbol)
    .bind(ask)
    .bind(bid)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get the latest forex rate for a symbol
pub async fn get_latest_forex_rate(
    pool: &SqlitePool,
    symbol: &str,
) -> Result<Option<(f64, f64, i64)>> {
    let record = sqlx::query_as::<_, (f64, f64, i64)>(
        r#"
        SELECT ask, bid, timestamp
        FROM forex_rates
        WHERE symbol = ?
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await?;

    Ok(record)
}

/// List all unique symbols in the forex_rates table
pub async fn list_forex_symbols(pool: &SqlitePool) -> Result<Vec<String>> {
    let records = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT DISTINCT symbol
        FROM forex_rates
        ORDER BY symbol
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().map(|(symbol,)| symbol).collect())
}

/// Update currencies from FMP API
pub async fn update_currencies(fmp_client: &FMPClient, pool: &SqlitePool) -> Result<()> {
    println!("Fetching currencies from FMP API...");
    let exchange_rates = match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            println!("✅ Currencies fetched");
            rates
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to fetch currencies: {}", e));
        }
    };

    // Extract unique currencies from exchange rates
    for rate in exchange_rates {
        if let Some(name) = rate.name {
            if let Some((from, to)) = name.split_once('/') {
                // Insert both currencies
                insert_currency(pool, from, from).await?;
                insert_currency(pool, to, to).await?;
            }
        }
    }

    println!("✅ Currencies updated in database");
    Ok(())
}

/// Get exchange rate info used for currency conversion
/// Returns the currency pair, exchange rate, and timestamp for a conversion from one currency to another
pub async fn get_conversion_rate_info(
    pool: &SqlitePool,
    from_currency: &str,
    to_currency: &str,
    at_timestamp: i64,
) -> Result<Option<(String, f64, i64)>> {
    // Handle special cases for currency subunits and alternative codes (same as convert_currency)
    let adjusted_from_currency = match from_currency {
        "GBp" => "GBP", // Convert pence to pounds
        "ZAc" => "ZAR",
        "ILA" => "ILS",
        _ => from_currency,
    };

    let adjusted_to_currency = match to_currency {
        "GBp" => "GBP",
        "ZAc" => "ZAR",
        "ILA" => "ILS",
        _ => to_currency,
    };

    if adjusted_from_currency == adjusted_to_currency {
        return Ok(Some((
            format!("{}/{}", adjusted_from_currency, adjusted_to_currency),
            1.0,
            at_timestamp,
        )));
    }

    // Try direct conversion first
    let direct_pair = format!("{}/{}", adjusted_from_currency, adjusted_to_currency);
    if let Some(rate_info) = get_forex_rate_at_timestamp(pool, &direct_pair, at_timestamp).await? {
        return Ok(Some((direct_pair, rate_info.0, rate_info.2)));
    }

    // Try reverse rate
    let reverse_pair = format!("{}/{}", adjusted_to_currency, adjusted_from_currency);
    if let Some(rate_info) = get_forex_rate_at_timestamp(pool, &reverse_pair, at_timestamp).await? {
        return Ok(Some((reverse_pair, 1.0 / rate_info.0, rate_info.2)));
    }

    // Try conversion through intermediate currencies - find most recent path
    let symbols = list_forex_symbols(pool).await?;
    for symbol in symbols {
        if let Some((from1, to1)) = symbol.split_once('/') {
            if from1 == adjusted_from_currency {
                let second_leg = format!("{}/{}", to1, adjusted_to_currency);
                if let Some(rate1_info) =
                    get_forex_rate_at_timestamp(pool, &symbol, at_timestamp).await?
                {
                    if let Some(rate2_info) =
                        get_forex_rate_at_timestamp(pool, &second_leg, at_timestamp).await?
                    {
                        let combined_rate = rate1_info.0 * rate2_info.0;
                        let combined_pair = format!(
                            "{}/{} (via {})",
                            adjusted_from_currency, adjusted_to_currency, to1
                        );
                        // Use the more recent timestamp
                        let timestamp = rate1_info.2.max(rate2_info.2);
                        return Ok(Some((combined_pair, combined_rate, timestamp)));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Get forex rate at or before a specific timestamp
async fn get_forex_rate_at_timestamp(
    pool: &SqlitePool,
    symbol: &str,
    at_timestamp: i64,
) -> Result<Option<(f64, f64, i64)>> {
    let record = sqlx::query_as::<_, (f64, f64, i64)>(
        r#"
        SELECT ask, bid, timestamp
        FROM forex_rates
        WHERE symbol = ? AND timestamp <= ?
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .bind(symbol)
    .bind(at_timestamp)
    .fetch_optional(pool)
    .await?;

    Ok(record)
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

        // Get all currency codes from rate_map
        let rate_map = get_rate_map_from_db(&pool).await?;
        let mut currencies: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Extract unique currency codes from rate pairs
        for pair in rate_map.keys() {
            if let Some((from, to)) = pair.split_once('/') {
                currencies.insert(from.to_string());
                currencies.insert(to.to_string());
            }
        }

        // Check if each currency exists in the database
        for currency in currencies {
            let result = list_currencies(&pool).await?;
            assert!(
                result.iter().any(|(c, _)| c == &currency),
                "Currency {} is used in rate_map but not found in database",
                currency
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_convert_currency() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        // Insert currencies
        insert_currency(&pool, "EUR", "Euro").await?;
        insert_currency(&pool, "USD", "US Dollar").await?;
        insert_currency(&pool, "JPY", "Japanese Yen").await?;
        insert_currency(&pool, "GBP", "British Pound").await?;
        insert_currency(&pool, "SEK", "Swedish Krona").await?;

        // Insert test forex rates
        insert_forex_rate(&pool, "EUR/USD", 1.08, 1.08, 1701956301).await?;
        insert_forex_rate(&pool, "USD/JPY", 150.0, 150.0, 1701956301).await?;
        insert_forex_rate(&pool, "GBP/USD", 1.25, 1.25, 1701956301).await?;
        insert_forex_rate(&pool, "EUR/SEK", 11.25, 11.25, 1701956301).await?;

        let rate_map = get_rate_map_from_db(&pool).await?;

        // Test direct USD conversions
        assert_eq!(
            convert_currency(100.0, "EUR", "USD", &rate_map),
            100.0 * 1.08
        );
        assert_eq!(
            convert_currency(100.0, "USD", "EUR", &rate_map),
            100.0 / 1.08
        );

        // Test cross rates between major currencies
        let eur_jpy = convert_currency(100.0, "EUR", "JPY", &rate_map);
        assert!(
            (eur_jpy - (100.0 * 1.08 * 150.0)).abs() < 0.01,
            "EUR/JPY rate for 100 EUR should be around {} JPY (got {})",
            100.0 * 1.08 * 150.0,
            eur_jpy
        );

        let gbp_jpy = convert_currency(100.0, "GBP", "JPY", &rate_map);
        assert!(
            (gbp_jpy - (100.0 * 1.25 * 150.0)).abs() < 0.01,
            "GBP/JPY rate for 100 GBP should be around {} JPY (got {})",
            100.0 * 1.25 * 150.0,
            gbp_jpy
        );

        // Test GBp (pence) conversion
        let gbp_eur = convert_currency(100.0, "GBP", "EUR", &rate_map);
        assert!(
            (gbp_eur - (100.0 * 1.25 / 1.08)).abs() < 0.01,
            "GBP/EUR rate should be around {} EUR (got {})",
            100.0 * 1.25 / 1.08,
            gbp_eur
        );

        // Test currencies with low unit value
        let sek_jpy = convert_currency(1000.0, "SEK", "JPY", &rate_map);
        assert!(
            (sek_jpy - (1000.0 / 11.25 * 1.08 * 150.0)).abs() < 0.01,
            "SEK/JPY rate for 1000 SEK should be around {} JPY (got {})",
            1000.0 / 11.25 * 1.08 * 150.0,
            sek_jpy
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_forex_rates() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

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

        // Test listing symbols
        let symbols = list_forex_symbols(&pool).await?;
        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains(&"EURUSD".to_string()));
        assert!(symbols.contains(&"GBPUSD".to_string()));

        // Test getting non-existent rate
        let missing = get_latest_forex_rate(&pool, "XXXYYY").await?;
        assert!(missing.is_none());

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
    async fn test_rate_map_from_db() -> Result<()> {
        let pool = db::create_db_pool("sqlite::memory:").await?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS forex_rates (
                symbol TEXT NOT NULL,
                ask REAL NOT NULL,
                bid REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Insert test data
        insert_forex_rate(&pool, "EUR/USD", 1.03128, 1.0321, 1736432800).await?;
        insert_forex_rate(&pool, "JPY/USD", 0.00675, 0.00674, 1736432800).await?;
        insert_forex_rate(&pool, "CHF/USD", 1.15, 1.149, 1736432800).await?;

        // Get rate map from db
        let rate_map = get_rate_map_from_db(&pool).await?;

        // Test direct rates
        assert_eq!(rate_map.get("EUR/USD"), Some(&1.03128));
        assert_eq!(rate_map.get("JPY/USD"), Some(&0.00675));
        assert_eq!(rate_map.get("CHF/USD"), Some(&1.15));

        // Test reverse rates
        assert!(
            (rate_map.get("USD/EUR").unwrap() - (1.0 / 1.03128)).abs() < 0.0001,
            "USD/EUR rate incorrect"
        );

        // Test cross rates
        let eur_jpy = rate_map.get("EUR/JPY").unwrap();
        let expected_eur_jpy = 1.03128 / 0.00675;
        assert!(
            (eur_jpy - expected_eur_jpy).abs() < 0.0001,
            "EUR/JPY rate incorrect: got {}, expected {}",
            eur_jpy,
            expected_eur_jpy
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_currency_operations() -> Result<()> {
        let db_url = "sqlite::memory:";
        let pool = crate::db::create_db_pool(db_url).await?;

        // Test inserting and retrieving a currency
        insert_currency(&pool, "XYZ", "Test Currency").await?;
        let currencies = list_currencies(&pool).await?;
        assert!(currencies.iter().any(|(c, _)| c == "XYZ"));

        // Test updating an existing currency
        insert_currency(&pool, "XYZ", "Updated Currency").await?;
        let updated = list_currencies(&pool).await?;
        assert!(updated
            .iter()
            .any(|(c, n)| c == "XYZ" && n == "Updated Currency"));

        // Test getting non-existent currency
        let missing = list_currencies(&pool).await?;
        assert!(!missing.iter().any(|(c, _)| c == "NON"));

        // Test listing currencies
        insert_currency(&pool, "ABC", "Another Currency").await?;
        let currencies = list_currencies(&pool).await?;
        assert_eq!(currencies.len(), 2);
        assert!(currencies.iter().any(|(c, _)| c == "XYZ"));
        assert!(currencies.iter().any(|(c, _)| c == "ABC"));

        Ok(())
    }
}
