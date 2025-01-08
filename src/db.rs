// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

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
