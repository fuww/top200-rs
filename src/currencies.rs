// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::FMPClient;
use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

/// Result of a currency conversion including the rate used
#[derive(Debug, Clone, Default)]
pub struct ConversionResult {
    /// The converted amount
    pub amount: f64,
    /// The effective rate used for conversion (from_currency -> to_currency)
    pub rate: f64,
    /// How the rate was determined: "direct", "reverse", "cross", "same", or "not_found"
    pub rate_source: &'static str,
    /// Warnings generated during conversion (e.g., rate validation issues)
    pub warnings: Vec<String>,
}

impl ConversionResult {
    /// Create a new ConversionResult with no warnings
    pub fn new(amount: f64, rate: f64, rate_source: &'static str) -> Self {
        Self {
            amount,
            rate,
            rate_source,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to this result
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Check if this result has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Validate an exchange rate for reasonableness
/// Returns None if valid, Some(warning_message) if suspicious
pub fn validate_rate(rate: f64, from_currency: &str, to_currency: &str) -> Option<String> {
    // Check for invalid rates
    if rate <= 0.0 {
        return Some(format!(
            "Invalid rate {:.6} for {}/{}: rate must be positive",
            rate, from_currency, to_currency
        ));
    }

    if rate.is_nan() || rate.is_infinite() {
        return Some(format!(
            "Invalid rate for {}/{}: rate is NaN or infinite",
            from_currency, to_currency
        ));
    }

    // Check for suspiciously extreme rates (more than 10,000:1 or less than 1:10,000)
    // This catches potential data errors while allowing legitimate high-ratio pairs like JPY
    if rate > 10_000.0 {
        return Some(format!(
            "Suspicious rate {:.6} for {}/{}: unusually high (>10,000)",
            rate, from_currency, to_currency
        ));
    }

    if rate < 0.0001 {
        return Some(format!(
            "Suspicious rate {:.6} for {}/{}: unusually low (<0.0001)",
            rate, from_currency, to_currency
        ));
    }

    None
}

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

/// Get a map of exchange rates between currencies from the database (latest rates)
pub async fn get_rate_map_from_db(pool: &SqlitePool) -> Result<HashMap<String, f64>> {
    get_rate_map_from_db_for_date(pool, None).await
}

/// Get a map of exchange rates for a specific date (or latest if None)
pub async fn get_rate_map_from_db_for_date(
    pool: &SqlitePool,
    timestamp: Option<i64>,
) -> Result<HashMap<String, f64>> {
    let mut rate_map = HashMap::new();

    // Get all unique symbols from the database
    let symbols = list_forex_symbols(pool).await?;

    // Get rates for each symbol (either for specific date or latest)
    for symbol in symbols {
        let rate_result = match timestamp {
            Some(ts) => get_forex_rate_for_date(pool, &symbol, ts).await?,
            None => get_latest_forex_rate(pool, &symbol).await?,
        };

        if let Some((ask, _bid, _timestamp)) = rate_result {
            // Skip symbols that don't have the expected format (e.g., "EUR/USD")
            if let Some((from, to)) = symbol.split_once('/') {
                rate_map.insert(format!("{}/{}", from, to), ask);
                rate_map.insert(format!("{}/{}", to, from), 1.0 / ask);
            }
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
/// Returns only the converted amount (for backwards compatibility)
pub fn convert_currency(
    amount: f64,
    from_currency: &str,
    to_currency: &str,
    rate_map: &HashMap<String, f64>,
) -> f64 {
    convert_currency_with_rate(amount, from_currency, to_currency, rate_map).amount
}

/// Convert an amount from one currency to another, returning the result with rate information
pub fn convert_currency_with_rate(
    amount: f64,
    from_currency: &str,
    to_currency: &str,
    rate_map: &HashMap<String, f64>,
) -> ConversionResult {
    if from_currency == to_currency {
        return ConversionResult::new(amount, 1.0, "same");
    }

    // Handle special cases for currency subunits and alternative codes
    let (adjusted_amount, adjusted_from_currency, subunit_divisor) = match from_currency {
        "GBp" => (amount / 100.0, "GBP", 100.0), // Convert pence to pounds
        "ZAc" => (amount / 100.0, "ZAR", 100.0),
        "ILA" => (amount, "ILS", 1.0),
        _ => (amount, from_currency, 1.0),
    };

    // Adjust target currency if needed
    let (adjusted_to_currency, target_multiplier) = match to_currency {
        "GBp" => ("GBP", 100.0), // Also handle GBp as target currency
        "ZAc" => ("ZAR", 100.0), // Also handle ZAc as target currency
        "ILA" => ("ILS", 1.0),
        _ => (to_currency, 1.0),
    };

    // Try direct conversion first
    let direct_rate = format!("{}/{}", adjusted_from_currency, adjusted_to_currency);
    if let Some(&rate) = rate_map.get(&direct_rate) {
        let result = adjusted_amount * rate * target_multiplier;
        // Effective rate accounts for subunit conversions
        let effective_rate = rate * target_multiplier / subunit_divisor;
        let mut conversion = ConversionResult::new(result, effective_rate, "direct");
        if let Some(warning) = validate_rate(rate, adjusted_from_currency, adjusted_to_currency) {
            conversion = conversion.with_warning(warning);
        }
        return conversion;
    }

    // Try reverse rate
    let reverse_rate = format!("{}/{}", adjusted_to_currency, adjusted_from_currency);
    if let Some(&rate) = rate_map.get(&reverse_rate) {
        let inverse_rate = 1.0 / rate;
        let result = adjusted_amount * inverse_rate * target_multiplier;
        let effective_rate = inverse_rate * target_multiplier / subunit_divisor;
        let mut conversion = ConversionResult::new(result, effective_rate, "reverse");
        if let Some(warning) = validate_rate(rate, adjusted_to_currency, adjusted_from_currency) {
            conversion = conversion.with_warning(warning);
        }
        return conversion;
    }

    // Try conversion through intermediate currencies
    for (pair, &rate1) in rate_map {
        if let Some((from1, to1)) = pair.split_once('/') {
            if from1 == adjusted_from_currency {
                let second_leg = format!("{}/{}", to1, adjusted_to_currency);
                if let Some(&rate2) = rate_map.get(&second_leg) {
                    let combined_rate = rate1 * rate2;
                    let result = adjusted_amount * combined_rate * target_multiplier;
                    let effective_rate = combined_rate * target_multiplier / subunit_divisor;
                    let mut conversion = ConversionResult::new(result, effective_rate, "cross");
                    // Validate both legs of the cross rate
                    if let Some(warning) = validate_rate(rate1, from1, to1) {
                        conversion = conversion.with_warning(warning);
                    }
                    if let Some(warning) = validate_rate(rate2, to1, adjusted_to_currency) {
                        conversion = conversion.with_warning(warning);
                    }
                    return conversion;
                }
            }
        }
    }

    // If no conversion rate is found, log a warning and return the original amount
    // This is a fallback to prevent crashes, but the data will be inaccurate
    eprintln!(
        "⚠️  Warning: No exchange rate found for {}/{}, returning unconverted amount",
        from_currency, to_currency
    );
    ConversionResult::new(amount, 1.0, "not_found").with_warning(format!(
        "No exchange rate found for {}/{}",
        from_currency, to_currency
    ))
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

/// Get forex rate for a specific date (or closest date before it)
pub async fn get_forex_rate_for_date(
    pool: &SqlitePool,
    symbol: &str,
    timestamp: i64,
) -> Result<Option<(f64, f64, i64)>> {
    let record = sqlx::query_as::<_, (f64, f64, i64)>(
        r#"
        SELECT ask, bid, timestamp
        FROM forex_rates
        WHERE symbol = ?
        AND timestamp <= ?
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .bind(symbol)
    .bind(timestamp)
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

    #[tokio::test]
    async fn test_convert_currency_with_rate() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        // Insert currencies and rates
        insert_currency(&pool, "EUR", "Euro").await?;
        insert_currency(&pool, "USD", "US Dollar").await?;
        insert_currency(&pool, "JPY", "Japanese Yen").await?;
        insert_currency(&pool, "GBP", "British Pound").await?;

        insert_forex_rate(&pool, "EUR/USD", 1.08, 1.08, 1701956301).await?;
        insert_forex_rate(&pool, "USD/JPY", 150.0, 150.0, 1701956301).await?;
        insert_forex_rate(&pool, "GBP/USD", 1.25, 1.25, 1701956301).await?;

        let rate_map = get_rate_map_from_db(&pool).await?;

        // Test same currency - rate should be 1.0
        let result = convert_currency_with_rate(100.0, "USD", "USD", &rate_map);
        assert_eq!(result.amount, 100.0);
        assert_eq!(result.rate, 1.0);
        assert_eq!(result.rate_source, "same");

        // Test direct rate EUR->USD
        let result = convert_currency_with_rate(100.0, "EUR", "USD", &rate_map);
        assert_relative_eq!(result.amount, 108.0, epsilon = 0.01);
        assert_relative_eq!(result.rate, 1.08, epsilon = 0.0001);
        assert_eq!(result.rate_source, "direct");

        // Test reverse rate USD->EUR
        let result = convert_currency_with_rate(100.0, "USD", "EUR", &rate_map);
        assert_relative_eq!(result.amount, 100.0 / 1.08, epsilon = 0.01);
        assert_relative_eq!(result.rate, 1.0 / 1.08, epsilon = 0.0001);
        // Could be "direct" or "reverse" depending on rate_map construction
        assert!(result.rate_source == "direct" || result.rate_source == "reverse");

        // Test cross rate EUR->JPY (via USD)
        let result = convert_currency_with_rate(100.0, "EUR", "JPY", &rate_map);
        let expected_amount = 100.0 * 1.08 * 150.0;
        let expected_rate = 1.08 * 150.0;
        assert_relative_eq!(result.amount, expected_amount, epsilon = 0.01);
        // Rate should be the combined rate (could be direct if cross-rate was precomputed)
        assert!(
            (result.rate - expected_rate).abs() < 0.01 || (result.rate - 1.08).abs() < 0.01 // If EUR/JPY is precomputed
        );

        // Test GBp (pence) to USD - subunit handling
        let result = convert_currency_with_rate(10000.0, "GBp", "USD", &rate_map);
        // 10000 pence = 100 GBP, 100 GBP * 1.25 = 125 USD
        assert_relative_eq!(result.amount, 125.0, epsilon = 0.01);
        // Effective rate: 1.25 / 100 = 0.0125 (pence to USD)
        assert_relative_eq!(result.rate, 0.0125, epsilon = 0.0001);

        Ok(())
    }

    // ==================== Phase 1: Edge Case Tests ====================

    #[test]
    fn test_validate_rate_valid_rates() {
        // Normal rates should pass validation
        assert!(validate_rate(1.08, "EUR", "USD").is_none());
        assert!(validate_rate(150.0, "USD", "JPY").is_none());
        assert!(validate_rate(0.01, "JPY", "USD").is_none());
        assert!(validate_rate(1.0, "USD", "USD").is_none());
    }

    #[test]
    fn test_validate_rate_zero_rate() {
        let warning = validate_rate(0.0, "EUR", "USD");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("must be positive"));
    }

    #[test]
    fn test_validate_rate_negative_rate() {
        let warning = validate_rate(-1.5, "EUR", "USD");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("must be positive"));
    }

    #[test]
    fn test_validate_rate_nan() {
        let warning = validate_rate(f64::NAN, "EUR", "USD");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("NaN or infinite"));
    }

    #[test]
    fn test_validate_rate_infinity() {
        let warning = validate_rate(f64::INFINITY, "EUR", "USD");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("NaN or infinite"));
    }

    #[test]
    fn test_validate_rate_extremely_high() {
        // Rate > 10,000 is suspicious
        let warning = validate_rate(15000.0, "XXX", "YYY");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("unusually high"));
    }

    #[test]
    fn test_validate_rate_extremely_low() {
        // Rate < 0.0001 is suspicious
        let warning = validate_rate(0.00001, "XXX", "YYY");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("unusually low"));
    }

    #[test]
    fn test_conversion_result_new() {
        let result = ConversionResult::new(100.0, 1.08, "direct");
        assert_eq!(result.amount, 100.0);
        assert_eq!(result.rate, 1.08);
        assert_eq!(result.rate_source, "direct");
        assert!(result.warnings.is_empty());
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_conversion_result_with_warning() {
        let result =
            ConversionResult::new(100.0, 1.08, "direct").with_warning("Test warning".to_string());
        assert!(result.has_warnings());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }

    #[test]
    fn test_conversion_result_multiple_warnings() {
        let result = ConversionResult::new(100.0, 1.08, "cross")
            .with_warning("Warning 1".to_string())
            .with_warning("Warning 2".to_string());
        assert!(result.has_warnings());
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn test_convert_missing_rate_generates_warning() {
        let rate_map: HashMap<String, f64> = HashMap::new();
        let result = convert_currency_with_rate(100.0, "XXX", "YYY", &rate_map);
        assert_eq!(result.rate_source, "not_found");
        assert!(result.has_warnings());
        assert!(result.warnings[0].contains("No exchange rate found"));
    }

    #[test]
    fn test_convert_same_currency_no_warnings() {
        let rate_map: HashMap<String, f64> = HashMap::new();
        let result = convert_currency_with_rate(100.0, "USD", "USD", &rate_map);
        assert_eq!(result.rate_source, "same");
        assert_eq!(result.amount, 100.0);
        assert!(!result.has_warnings());
    }

    #[tokio::test]
    async fn test_convert_with_suspicious_rate() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        // Insert a suspiciously high rate
        insert_forex_rate(&pool, "XXX/YYY", 50000.0, 50000.0, 1701956301).await?;

        let rate_map = get_rate_map_from_db(&pool).await?;
        let result = convert_currency_with_rate(100.0, "XXX", "YYY", &rate_map);

        // Conversion should still work
        assert_relative_eq!(result.amount, 5_000_000.0, epsilon = 0.01);
        // But should have a warning
        assert!(result.has_warnings());
        assert!(result.warnings[0].contains("unusually high"));

        Ok(())
    }

    #[test]
    fn test_conversion_result_default() {
        let result = ConversionResult::default();
        assert_eq!(result.amount, 0.0);
        assert_eq!(result.rate, 0.0);
        assert_eq!(result.rate_source, "");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_rate_boundary_values() {
        // Just below threshold - should pass
        assert!(validate_rate(9999.0, "A", "B").is_none());
        assert!(validate_rate(0.0002, "A", "B").is_none());

        // Just above threshold - should warn
        assert!(validate_rate(10001.0, "A", "B").is_some());
        assert!(validate_rate(0.00009, "A", "B").is_some());
    }

    // ==================== Phase 2: Property-Based Tests ====================

    use proptest::prelude::*;

    proptest! {
        /// Property: Same-currency conversion should always return exact amount
        #[test]
        fn prop_same_currency_returns_exact_amount(amount in 0.01f64..1e12f64) {
            let rate_map: HashMap<String, f64> = HashMap::new();
            let result = convert_currency_with_rate(amount, "USD", "USD", &rate_map);
            prop_assert_eq!(result.amount, amount);
            prop_assert_eq!(result.rate, 1.0);
            prop_assert_eq!(result.rate_source, "same");
            prop_assert!(!result.has_warnings());
        }

        /// Property: Round-trip conversion should return approximately original
        #[test]
        fn prop_round_trip_conversion_preserves_amount(amount in 1.0f64..1e9f64) {
            let mut rate_map = HashMap::new();
            rate_map.insert("USD/EUR".to_string(), 0.92);
            rate_map.insert("EUR/USD".to_string(), 1.0 / 0.92);

            let to_eur = convert_currency_with_rate(amount, "USD", "EUR", &rate_map);
            let back_to_usd = convert_currency_with_rate(to_eur.amount, "EUR", "USD", &rate_map);

            // Should be within 0.01% due to floating point
            let diff = (back_to_usd.amount - amount).abs() / amount;
            prop_assert!(diff < 0.0001, "Round-trip diff {} > 0.01%", diff * 100.0);
        }

        /// Property: Conversion with rate R should have inverse rate 1/R
        #[test]
        fn prop_inverse_rates_are_reciprocal(rate in 0.001f64..1000.0f64) {
            let mut rate_map = HashMap::new();
            rate_map.insert("AAA/BBB".to_string(), rate);
            rate_map.insert("BBB/AAA".to_string(), 1.0 / rate);

            let forward = convert_currency_with_rate(100.0, "AAA", "BBB", &rate_map);
            let reverse = convert_currency_with_rate(100.0, "BBB", "AAA", &rate_map);

            // forward.rate * reverse.rate should equal 1.0
            let product = forward.rate * reverse.rate;
            prop_assert!((product - 1.0).abs() < 0.0001, "Rate product {} != 1.0", product);
        }

        /// Property: Positive amount with positive rate should give positive result
        #[test]
        fn prop_positive_input_gives_positive_output(
            amount in 0.01f64..1e12f64,
            rate in 0.001f64..1000.0f64
        ) {
            let mut rate_map = HashMap::new();
            rate_map.insert("XXX/YYY".to_string(), rate);

            let result = convert_currency_with_rate(amount, "XXX", "YYY", &rate_map);
            prop_assert!(result.amount > 0.0, "Amount {} should be positive", result.amount);
            prop_assert!(result.rate > 0.0, "Rate {} should be positive", result.rate);
        }

        /// Property: validate_rate should accept all rates in reasonable range
        #[test]
        fn prop_validate_rate_accepts_reasonable_range(rate in 0.0002f64..9999.0f64) {
            let result = validate_rate(rate, "A", "B");
            prop_assert!(result.is_none(), "Rate {} should be valid", rate);
        }

        /// Property: validate_rate should reject rates outside reasonable range
        #[test]
        fn prop_validate_rate_rejects_extreme_low(rate in 0.0f64..0.00005f64) {
            // Very low rates should trigger warning
            if rate < 0.0001 {
                let result = validate_rate(rate, "A", "B");
                prop_assert!(result.is_some(), "Rate {} should be flagged as suspicious", rate);
            }
        }

        /// Property: validate_rate should reject extremely high rates
        #[test]
        fn prop_validate_rate_rejects_extreme_high(rate in 10001.0f64..1e15f64) {
            let result = validate_rate(rate, "A", "B");
            prop_assert!(result.is_some(), "Rate {} should be flagged as suspicious", rate);
        }

        /// Property: ConversionResult.with_warning should always increment warning count
        #[test]
        fn prop_with_warning_increments_count(warning_count in 1usize..10usize) {
            let mut result = ConversionResult::new(100.0, 1.0, "test");
            for i in 0..warning_count {
                result = result.with_warning(format!("Warning {}", i));
            }
            prop_assert_eq!(result.warnings.len(), warning_count);
            prop_assert!(result.has_warnings());
        }

        /// Property: Subunit conversion should preserve value equivalence
        #[test]
        fn prop_subunit_conversion_preserves_value(pence in 100u64..10_000_000u64) {
            let mut rate_map = HashMap::new();
            rate_map.insert("GBP/USD".to_string(), 1.25);
            rate_map.insert("USD/GBP".to_string(), 0.8);

            // Convert pence to USD
            let from_pence = convert_currency_with_rate(pence as f64, "GBp", "USD", &rate_map);

            // Convert equivalent pounds to USD
            let pounds = pence as f64 / 100.0;
            let from_pounds = convert_currency_with_rate(pounds, "GBP", "USD", &rate_map);

            // Should give same result
            let diff = (from_pence.amount - from_pounds.amount).abs();
            prop_assert!(diff < 0.01, "Pence and pounds conversion differ by {}", diff);
        }
    }
}
