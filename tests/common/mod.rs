// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Common test utilities and helpers
//!
//! This module provides reusable test infrastructure for all integration and unit tests.
//! It includes:
//! - Database creation with pre-populated test data
//! - Standard rate maps for currency conversion testing
//! - CSV file generation and validation helpers

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use csv::Writer;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Creates a temporary SQLite database with basic schema and test data
pub async fn create_test_db() -> Result<(SqlitePool, TempDir)> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok((pool, temp_dir))
}

/// Creates a test database populated with sample exchange rates
pub async fn create_test_db_with_rates() -> Result<(SqlitePool, TempDir)> {
    let (pool, temp_dir) = create_test_db().await?;

    // Insert sample currencies
    let currencies = vec![
        ("USD", "US Dollar"),
        ("EUR", "Euro"),
        ("GBP", "British Pound"),
        ("JPY", "Japanese Yen"),
        ("CHF", "Swiss Franc"),
        ("SEK", "Swedish Krona"),
    ];

    for (code, name) in currencies {
        sqlx::query!(
            "INSERT OR IGNORE INTO currencies (code, name) VALUES (?, ?)",
            code,
            name
        )
        .execute(&pool)
        .await?;
    }

    // Insert sample exchange rates (current timestamp)
    let timestamp = chrono::Utc::now().timestamp();
    let rates = create_sample_rate_map();

    for (symbol, rate) in rates.iter() {
        sqlx::query!(
            "INSERT INTO forex_rates (symbol, ask, bid, timestamp) VALUES (?, ?, ?, ?)",
            symbol,
            rate,
            rate, // Use same rate for bid/ask in tests
            timestamp
        )
        .execute(&pool)
        .await?;
    }

    Ok((pool, temp_dir))
}

/// Creates a standard rate map for testing currency conversions
///
/// This includes common currency pairs with realistic rates:
/// - EUR/USD: ~1.08
/// - GBP/USD: ~1.27
/// - USD/JPY: ~150.0
/// - EUR/GBP: ~0.85
/// - And several cross rates
pub fn create_sample_rate_map() -> HashMap<String, f64> {
    let mut rate_map = HashMap::new();

    // Major currency pairs
    rate_map.insert("EUR/USD".to_string(), 1.08);
    rate_map.insert("USD/EUR".to_string(), 1.0 / 1.08);
    rate_map.insert("GBP/USD".to_string(), 1.27);
    rate_map.insert("USD/GBP".to_string(), 1.0 / 1.27);
    rate_map.insert("USD/JPY".to_string(), 150.0);
    rate_map.insert("JPY/USD".to_string(), 1.0 / 150.0);
    rate_map.insert("EUR/GBP".to_string(), 0.85);
    rate_map.insert("GBP/EUR".to_string(), 1.0 / 0.85);
    rate_map.insert("USD/CHF".to_string(), 0.88);
    rate_map.insert("CHF/USD".to_string(), 1.0 / 0.88);
    rate_map.insert("USD/SEK".to_string(), 10.5);
    rate_map.insert("SEK/USD".to_string(), 1.0 / 10.5);

    // Cross rates for testing transitivity
    rate_map.insert("EUR/JPY".to_string(), 1.08 * 150.0); // 162.0
    rate_map.insert("GBP/JPY".to_string(), 1.27 * 150.0); // 190.5

    rate_map
}

/// Creates a test CSV file with market cap data
///
/// Returns the path to the created temporary CSV file.
/// The caller is responsible for cleaning up the file.
pub fn create_test_csv_file(
    dir: &Path,
    date: &str,
    companies: &[TestCompany],
) -> Result<PathBuf> {
    let filename = format!("marketcaps_{}_{}.csv", date, "test");
    let file_path = dir.join(&filename);
    let file = std::fs::File::create(&file_path)?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Rank",
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "EUR Rate",
        "Market Cap (USD)",
        "USD Rate",
        "Price",
        "Exchange",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "CEO",
        "Date",
    ])?;

    // Write data
    for (index, company) in companies.iter().enumerate() {
        writer.write_record(&[
            (index + 1).to_string(),
            company.ticker.clone(),
            company.name.clone(),
            format!("{:.0}", company.market_cap),
            company.currency.clone(),
            format!("{:.0}", company.market_cap_eur),
            format!("{:.6}", company.eur_rate),
            format!("{:.0}", company.market_cap_usd),
            format!("{:.6}", company.usd_rate),
            company.price.to_string(),
            company.exchange.clone(),
            "true".to_string(),
            company.description.clone().unwrap_or_default(),
            company.homepage.clone().unwrap_or_default(),
            company.employees.map(|e| e.to_string()).unwrap_or_default(),
            company.ceo.clone().unwrap_or_default(),
            date.to_string(),
        ])?;
    }

    writer.flush()?;
    Ok(file_path)
}

/// Test company data structure for creating test CSV files
#[derive(Debug, Clone)]
pub struct TestCompany {
    pub ticker: String,
    pub name: String,
    pub market_cap: f64,
    pub currency: String,
    pub market_cap_eur: f64,
    pub eur_rate: f64,
    pub market_cap_usd: f64,
    pub usd_rate: f64,
    pub price: f64,
    pub exchange: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub employees: Option<i32>,
    pub ceo: Option<String>,
}

impl TestCompany {
    /// Create a simple test company with minimal data
    pub fn simple(ticker: &str, name: &str, market_cap_usd: f64) -> Self {
        let eur_rate = 1.08; // USD/EUR rate
        Self {
            ticker: ticker.to_string(),
            name: name.to_string(),
            market_cap: market_cap_usd,
            currency: "USD".to_string(),
            market_cap_eur: market_cap_usd * eur_rate,
            eur_rate,
            market_cap_usd,
            usd_rate: 1.0,
            price: 100.0,
            exchange: "NASDAQ".to_string(),
            description: None,
            homepage: None,
            employees: None,
            ceo: None,
        }
    }
}

/// Validates that a CSV file has the expected columns
pub fn assert_csv_has_columns(csv_path: &Path, expected_columns: &[&str]) -> Result<()> {
    let file = std::fs::File::open(csv_path)?;
    let mut reader = csv::Reader::from_reader(file);

    let headers = reader.headers()?;
    let header_vec: Vec<&str> = headers.iter().collect();

    for expected in expected_columns {
        if !header_vec.contains(expected) {
            anyhow::bail!("Missing expected column: {}", expected);
        }
    }

    Ok(())
}

/// Creates a timestamp from a date string (YYYY-MM-DD)
pub fn date_to_timestamp(date_str: &str) -> Result<i64> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
    let naive_dt = NaiveDateTime::new(date, NaiveTime::default());
    Ok(naive_dt.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_test_db() {
        let result = create_test_db().await;
        assert!(result.is_ok(), "Should create test database");

        let (pool, _temp_dir) = result.unwrap();

        // Verify we can query the database
        let result = sqlx::query!("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(&pool)
            .await;
        assert!(result.is_ok(), "Should be able to query database");
    }

    #[tokio::test]
    async fn test_create_test_db_with_rates() {
        let result = create_test_db_with_rates().await;
        assert!(result.is_ok(), "Should create database with rates");

        let (pool, _temp_dir) = result.unwrap();

        // Verify currencies were inserted
        let currencies = sqlx::query!("SELECT COUNT(*) as count FROM currencies")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(currencies.count > 0, "Should have currencies");

        // Verify rates were inserted
        let rates = sqlx::query!("SELECT COUNT(*) as count FROM forex_rates")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(rates.count > 0, "Should have exchange rates");
    }

    #[test]
    fn test_create_sample_rate_map() {
        let rate_map = create_sample_rate_map();

        // Should have major pairs
        assert!(rate_map.contains_key("EUR/USD"));
        assert!(rate_map.contains_key("GBP/USD"));
        assert!(rate_map.contains_key("USD/JPY"));

        // Should have reverse pairs
        assert!(rate_map.contains_key("USD/EUR"));
        assert!(rate_map.contains_key("USD/GBP"));

        // Verify rate reciprocals
        let eur_usd = rate_map.get("EUR/USD").unwrap();
        let usd_eur = rate_map.get("USD/EUR").unwrap();
        let product = eur_usd * usd_eur;
        assert!((product - 1.0).abs() < 0.0001, "Reverse rates should be reciprocals");
    }

    #[test]
    fn test_create_test_csv_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let companies = vec![
            TestCompany::simple("AAPL", "Apple Inc.", 3_000_000_000_000.0),
            TestCompany::simple("MSFT", "Microsoft Corp.", 2_500_000_000_000.0),
        ];

        let result = create_test_csv_file(temp_dir.path(), "2025-01-01", &companies);
        assert!(result.is_ok(), "Should create CSV file");

        let csv_path = result.unwrap();
        assert!(csv_path.exists(), "CSV file should exist");

        // Verify columns
        let columns = assert_csv_has_columns(
            &csv_path,
            &["Rank", "Ticker", "Name", "Market Cap (EUR)", "Market Cap (USD)"],
        );
        assert!(columns.is_ok(), "Should have expected columns");
    }

    #[test]
    fn test_test_company_simple() {
        let company = TestCompany::simple("NKE", "Nike Inc.", 100_000_000_000.0);

        assert_eq!(company.ticker, "NKE");
        assert_eq!(company.name, "Nike Inc.");
        assert_eq!(company.market_cap_usd, 100_000_000_000.0);
        assert_eq!(company.currency, "USD");
        assert_eq!(company.usd_rate, 1.0);
    }

    #[test]
    fn test_date_to_timestamp() {
        let result = date_to_timestamp("2025-01-01");
        assert!(result.is_ok(), "Should parse valid date");

        let timestamp = result.unwrap();
        assert!(timestamp > 1_700_000_000, "Timestamp should be after 2023");
        assert!(timestamp < 2_000_000_000, "Timestamp should be before 2033");
    }
}
