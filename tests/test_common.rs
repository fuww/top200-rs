// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for common test utilities

mod common;

use common::*;

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
    assert!(
        (product - 1.0).abs() < 0.0001,
        "Reverse rates should be reciprocals"
    );
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
        &[
            "Rank",
            "Ticker",
            "Name",
            "Market Cap (EUR)",
            "Market Cap (USD)",
        ],
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
