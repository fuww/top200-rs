// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Sqlite};
use crate::api::ExchangeRate;

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

pub async fn store_forex_rates(pool: &SqlitePool, rates: &[ExchangeRate]) -> Result<()> {
    for rate in rates {
        // Skip if we don't have the required fields
        let (Some(name), Some(bid), Some(ask)) = (
            rate.name.as_ref(),
            rate.previous_close,  // Using previous_close as bid
            rate.price,          // Using current price as ask
        ) else {
            continue;
        };

        sqlx::query!(
            r#"
            INSERT INTO forex_rates (symbol, bid, ask, timestamp)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(symbol, timestamp) DO UPDATE SET
                bid = excluded.bid,
                ask = excluded.ask
            "#,
            name,
            bid,
            ask,
            rate.timestamp
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
