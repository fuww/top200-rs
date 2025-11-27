// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

#[derive(Debug)]
pub struct TickerDetails {
    pub ticker: String,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub employees: Option<String>,
    pub ceo: Option<String>,
}

/// Update ticker details in the database
pub async fn update_ticker_details(pool: &SqlitePool, details: &TickerDetails) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO ticker_details (ticker, description, homepage_url, employees, ceo)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(ticker) DO UPDATE SET
            description = excluded.description,
            homepage_url = excluded.homepage_url,
            employees = excluded.employees,
            ceo = excluded.ceo,
            updated_at = CURRENT_TIMESTAMP
        "#,
        details.ticker,
        details.description,
        details.homepage_url,
        details.employees,
        details.ceo,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::create_db_pool;
    use sqlx::Row;

    // Test the TickerDetails struct
    #[test]
    fn test_ticker_details_struct_creation() {
        let details = TickerDetails {
            ticker: "AAPL".to_string(),
            description: Some(
                "Apple Inc. designs, manufactures, and markets smartphones".to_string(),
            ),
            homepage_url: Some("https://apple.com".to_string()),
            employees: Some("164000".to_string()),
            ceo: Some("Tim Cook".to_string()),
        };

        assert_eq!(details.ticker, "AAPL");
        assert!(details.description.as_ref().unwrap().contains("Apple"));
        assert_eq!(details.homepage_url, Some("https://apple.com".to_string()));
        assert_eq!(details.employees, Some("164000".to_string()));
        assert_eq!(details.ceo, Some("Tim Cook".to_string()));
    }

    #[test]
    fn test_ticker_details_with_all_none_fields() {
        let details = TickerDetails {
            ticker: "XYZ".to_string(),
            description: None,
            homepage_url: None,
            employees: None,
            ceo: None,
        };

        assert_eq!(details.ticker, "XYZ");
        assert!(details.description.is_none());
        assert!(details.homepage_url.is_none());
        assert!(details.employees.is_none());
        assert!(details.ceo.is_none());
    }

    #[test]
    fn test_ticker_details_debug() {
        let details = TickerDetails {
            ticker: "AAPL".to_string(),
            description: Some("Apple Inc.".to_string()),
            homepage_url: None,
            employees: Some("164000".to_string()),
            ceo: Some("Tim Cook".to_string()),
        };

        let debug_str = format!("{:?}", details);
        assert!(debug_str.contains("AAPL"));
        assert!(debug_str.contains("Tim Cook"));
    }

    #[test]
    fn test_ticker_details_with_special_characters() {
        let details = TickerDetails {
            ticker: "HM-B.ST".to_string(), // Special characters in ticker
            description: Some("H&M Hennes & Mauritz AB".to_string()), // Ampersand
            homepage_url: Some("https://hm.com/en_gb/".to_string()),
            employees: Some("100000".to_string()),
            ceo: Some("Helena Helmersson".to_string()),
        };

        assert_eq!(details.ticker, "HM-B.ST");
        assert!(details.description.as_ref().unwrap().contains("H&M"));
        assert!(details.homepage_url.as_ref().unwrap().contains("hm.com"));
    }

    #[test]
    fn test_ticker_details_clone_like_behavior() {
        let details1 = TickerDetails {
            ticker: "MSFT".to_string(),
            description: Some("Microsoft Corporation".to_string()),
            homepage_url: Some("https://microsoft.com".to_string()),
            employees: Some("200000".to_string()),
            ceo: Some("Satya Nadella".to_string()),
        };

        // Test that we can create another struct with same values
        let details2 = TickerDetails {
            ticker: details1.ticker.clone(),
            description: details1.description.clone(),
            homepage_url: details1.homepage_url.clone(),
            employees: details1.employees.clone(),
            ceo: details1.ceo.clone(),
        };

        assert_eq!(details1.ticker, details2.ticker);
        assert_eq!(details1.description, details2.description);
        assert_eq!(details1.ceo, details2.ceo);
    }

    // Test database schema exists
    #[tokio::test]
    async fn test_ticker_details_table_exists() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Check that the ticker_details table exists
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='ticker_details'",
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to query sqlite_master");

        assert!(result.is_some(), "ticker_details table should exist");
    }

    #[tokio::test]
    async fn test_ticker_details_table_schema() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Check the table schema using PRAGMA
        let rows = sqlx::query("PRAGMA table_info(ticker_details)")
            .fetch_all(&pool)
            .await
            .expect("Failed to get table info");

        // Should have at least 5 columns: ticker, description, homepage_url, employees, updated_at
        assert!(rows.len() >= 5);

        // Check column names
        let column_names: Vec<String> = rows.iter().map(|r| r.get("name")).collect();
        assert!(column_names.contains(&"ticker".to_string()));
        assert!(column_names.contains(&"description".to_string()));
        assert!(column_names.contains(&"homepage_url".to_string()));
        assert!(column_names.contains(&"employees".to_string()));
    }

    #[tokio::test]
    async fn test_ticker_details_direct_insert() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Insert directly with raw SQL to test the schema
        // Note: ceo column may not exist in all migrations, use only columns we know exist
        sqlx::query(
            "INSERT INTO ticker_details (ticker, description, homepage_url, employees) VALUES ('TEST', 'Test Company', 'https://test.com', 100)"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test ticker");

        // Verify it was inserted
        let row = sqlx::query(
            "SELECT ticker, description, homepage_url FROM ticker_details WHERE ticker = 'TEST'",
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch");

        let ticker: String = row.get("ticker");
        let description: Option<String> = row.get("description");
        let homepage_url: Option<String> = row.get("homepage_url");

        assert_eq!(ticker, "TEST");
        assert_eq!(description, Some("Test Company".to_string()));
        assert_eq!(homepage_url, Some("https://test.com".to_string()));
    }

    #[tokio::test]
    async fn test_ticker_details_upsert_behavior() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Insert initial record
        sqlx::query(
            "INSERT INTO ticker_details (ticker, description, homepage_url) VALUES ('UPSERT', 'Initial', 'https://initial.com')"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert initial");

        // Upsert with new values
        sqlx::query(
            "INSERT INTO ticker_details (ticker, description, homepage_url) VALUES ('UPSERT', 'Updated', 'https://updated.com') ON CONFLICT(ticker) DO UPDATE SET description = excluded.description, homepage_url = excluded.homepage_url"
        )
        .execute(&pool)
        .await
        .expect("Failed to upsert");

        // Verify only one record exists with updated values
        let count: i32 =
            sqlx::query("SELECT COUNT(*) as cnt FROM ticker_details WHERE ticker = 'UPSERT'")
                .fetch_one(&pool)
                .await
                .map(|row| row.get("cnt"))
                .expect("Failed to count");

        assert_eq!(count, 1);

        let row = sqlx::query(
            "SELECT description, homepage_url FROM ticker_details WHERE ticker = 'UPSERT'",
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch");

        let description: Option<String> = row.get("description");
        let homepage_url: Option<String> = row.get("homepage_url");

        assert_eq!(description, Some("Updated".to_string()));
        assert_eq!(homepage_url, Some("https://updated.com".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_tickers_direct_insert() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Insert multiple tickers
        for (ticker, desc) in [("NKE", "Nike"), ("LULU", "Lululemon"), ("GPS", "Gap")] {
            sqlx::query("INSERT INTO ticker_details (ticker, description) VALUES (?, ?)")
                .bind(ticker)
                .bind(desc)
                .execute(&pool)
                .await
                .expect("Failed to insert");
        }

        // Verify all were inserted
        let count: i32 = sqlx::query("SELECT COUNT(*) as cnt FROM ticker_details")
            .fetch_one(&pool)
            .await
            .map(|row| row.get("cnt"))
            .expect("Failed to count");

        assert_eq!(count, 3);
    }
}
