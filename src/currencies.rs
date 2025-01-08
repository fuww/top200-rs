// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

/// Get a currency from the database by its code
pub async fn get_currency(pool: &SqlitePool, code: &str) -> Result<Option<(String, String)>> {
    let record = sqlx::query!(
        r#"
        SELECT code, name
        FROM currencies
        WHERE code = ?
        "#,
        code
    )
    .fetch_optional(pool)
    .await?;

    Ok(record.map(|r| (r.code.unwrap_or_default(), r.name)))
}

/// List all currencies in the database
pub async fn list_currencies(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    let records = sqlx::query!(
        r#"
        SELECT code, name
        FROM currencies
        ORDER BY code
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(records.into_iter().map(|r| (r.code.unwrap_or_default(), r.name)).collect())
}

/// Get a map of exchange rates between currencies
pub fn get_rate_map() -> HashMap<String, f64> {
    let mut rate_map = HashMap::new();

    // Base rates (currency to USD)
    rate_map.insert("EUR/USD".to_string(), 1.08);
    rate_map.insert("GBP/USD".to_string(), 1.25);
    rate_map.insert("CHF/USD".to_string(), 1.14);
    rate_map.insert("SEK/USD".to_string(), 0.096);
    rate_map.insert("DKK/USD".to_string(), 0.145);
    rate_map.insert("NOK/USD".to_string(), 0.093);
    rate_map.insert("JPY/USD".to_string(), 0.0068);
    rate_map.insert("HKD/USD".to_string(), 0.128);
    rate_map.insert("CNY/USD".to_string(), 0.139);
    rate_map.insert("BRL/USD".to_string(), 0.203);
    rate_map.insert("CAD/USD".to_string(), 0.737);
    rate_map.insert("ILS/USD".to_string(), 0.27); // Israeli Shekel rate
    rate_map.insert("ZAR/USD".to_string(), 0.053); // South African Rand rate

    // Add reverse rates (USD to currency)
    let mut pairs_to_add = Vec::new();
    for (pair, &rate) in rate_map.clone().iter() {
        if let Some((from, to)) = pair.split_once('/') {
            pairs_to_add.push((format!("{}/{}", to, from), 1.0 / rate));
        }
    }

    // Add cross rates (currency to currency)
    let base_pairs: Vec<_> = rate_map.clone().into_iter().collect();
    for (pair1, rate1) in &base_pairs {
        if let Some((from1, "USD")) = pair1.split_once('/') {
            for (pair2, rate2) in &base_pairs {
                if let Some(("USD", to2)) = pair2.split_once('/') {
                    if from1 != to2 {
                        // Calculate cross rate: from1/to2 = (from1/USD) * (USD/to2)
                        pairs_to_add.push((format!("{}/{}", from1, to2), rate1 * rate2));
                    }
                }
            }
        }
    }

    // Add all the new pairs
    for (pair, rate) in pairs_to_add {
        rate_map.insert(pair, rate);
    }

    rate_map
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

    // If no conversion rate is found, return the original amount
    amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crate::db;

    #[tokio::test]
    async fn test_currencies_in_database() -> Result<()> {
        // Set up database connection
        let db_url = "sqlite::memory:";  // Use in-memory database for testing
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
        let rate_map = get_rate_map();
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
            let result = get_currency(&pool, &currency).await?;
            assert!(
                result.is_some(),
                "Currency {} is used in rate_map but not found in database",
                currency
            );
        }

        Ok(())
    }

    #[test]
    fn test_convert_currency() {
        let mut rate_map = HashMap::new();
        rate_map.insert("EUR/USD".to_string(), 1.08);
        rate_map.insert("USD/EUR".to_string(), 0.9259259259259258);

        // Test direct conversion
        let result = convert_currency(100.0, "EUR", "USD", &rate_map);
        assert_relative_eq!(result, 108.0, epsilon = 0.01);

        // Test reverse conversion
        let result = convert_currency(108.0, "USD", "EUR", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);

        // Test same currency
        let result = convert_currency(100.0, "USD", "USD", &rate_map);
        assert_relative_eq!(result, 100.0, epsilon = 0.01);
    }
}
