// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::sqlite::SqlitePool;

pub async fn create_db_pool(db_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(db_url).await?;
    
    // Create the currencies table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS currencies (
            code TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
