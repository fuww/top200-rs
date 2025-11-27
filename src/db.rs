// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

// use crate::api::ExchangeRate;
// use crate::currencies;
use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Sqlite};

pub async fn create_db_pool(db_url: &str) -> Result<SqlitePool> {
    // Create database if it doesn't exist
    if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
        Sqlite::create_database(db_url).await?;
    }

    // Connect to the database
    let pool = SqlitePool::connect(db_url).await?;

    // Run migrations
    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[tokio::test]
    async fn test_create_in_memory_db_pool() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Verify the pool is valid by executing a simple query
        let row = sqlx::query("SELECT 1 as val")
            .fetch_one(&pool)
            .await
            .expect("Failed to execute query");

        let val: i32 = row.get("val");
        assert_eq!(val, 1);
    }

    #[tokio::test]
    async fn test_migrations_create_currencies_table() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Check that the currencies table exists
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='currencies'")
                .fetch_optional(&pool)
                .await
                .expect("Failed to query sqlite_master");

        assert!(result.is_some(), "currencies table should exist");
    }

    #[tokio::test]
    async fn test_migrations_create_forex_rates_table() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Check that the forex_rates table exists
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='forex_rates'")
                .fetch_optional(&pool)
                .await
                .expect("Failed to query sqlite_master");

        assert!(result.is_some(), "forex_rates table should exist");
    }

    #[tokio::test]
    async fn test_migrations_create_market_caps_table() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Check that the market_caps table exists
        let result =
            sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='market_caps'")
                .fetch_optional(&pool)
                .await
                .expect("Failed to query sqlite_master");

        assert!(result.is_some(), "market_caps table should exist");
    }

    #[tokio::test]
    async fn test_migrations_create_ticker_details_table() {
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
    async fn test_migrations_create_symbol_changes_table() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Check that the symbol_changes table exists
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='symbol_changes'",
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to query sqlite_master");

        assert!(result.is_some(), "symbol_changes table should exist");
    }

    #[tokio::test]
    async fn test_can_insert_and_query_currency() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Insert a currency
        sqlx::query("INSERT INTO currencies (code, name) VALUES ('USD', 'US Dollar')")
            .execute(&pool)
            .await
            .expect("Failed to insert currency");

        // Query it back
        let row = sqlx::query("SELECT code, name FROM currencies WHERE code = 'USD'")
            .fetch_one(&pool)
            .await
            .expect("Failed to query currency");

        let code: String = row.get("code");
        let name: String = row.get("name");

        assert_eq!(code, "USD");
        assert_eq!(name, "US Dollar");
    }

    #[tokio::test]
    async fn test_multiple_pool_connections() {
        let pool = create_db_pool("sqlite::memory:")
            .await
            .expect("Failed to create database");

        // Run multiple queries concurrently
        let (r1, r2, r3) = tokio::join!(
            sqlx::query("SELECT 1 as val").fetch_one(&pool),
            sqlx::query("SELECT 2 as val").fetch_one(&pool),
            sqlx::query("SELECT 3 as val").fetch_one(&pool),
        );

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());

        let v1: i32 = r1.unwrap().get("val");
        let v2: i32 = r2.unwrap().get("val");
        let v3: i32 = r3.unwrap().get("val");

        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
        assert_eq!(v3, 3);
    }
}
