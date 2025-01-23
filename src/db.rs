// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

pub async fn create_db_pool(db_url: &str) -> Result<SqlitePool> {
    // If the URL is a file path (not :memory:), ensure the directory exists
    if !db_url.contains(":memory:") {
        if let Some(path) = db_url.strip_prefix("sqlite:") {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Touch the file to create it
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(path)?;
        }
    }

    // Use SqliteConnectOptions to create the database if it doesn't exist
    let options = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_db_pool() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.to_str().unwrap());

        let pool = create_db_pool(&db_url).await?;
        assert!(pool.acquire().await.is_ok());

        Ok(())
    }
}
