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
}

/// Update ticker details in the database
pub async fn update_ticker_details(pool: &SqlitePool, details: &TickerDetails) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO ticker_details (ticker, description, homepage_url, employees)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(ticker) DO UPDATE SET
            description = excluded.description,
            homepage_url = excluded.homepage_url,
            employees = excluded.employees,
            updated_at = CURRENT_TIMESTAMP
        "#,
        details.ticker,
        details.description,
        details.homepage_url,
        details.employees,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get ticker details from the database
pub async fn get_ticker_details(pool: &SqlitePool, ticker: &str) -> Result<Option<TickerDetails>> {
    let record = sqlx::query!(
        r#"
        SELECT ticker, description, homepage_url, employees
        FROM ticker_details
        WHERE ticker = ?
        "#,
        ticker
    )
    .fetch_optional(pool)
    .await?;

    Ok(record.map(|r| TickerDetails {
        ticker: r.ticker.unwrap_or_default(),
        description: r.description,
        homepage_url: r.homepage_url,
        employees: r.employees.map(|e| e.to_string()),
    }))
}

/// List all ticker details
pub async fn list_ticker_details(pool: &SqlitePool) -> Result<Vec<TickerDetails>> {
    let records = sqlx::query!(
        r#"
        SELECT ticker, description, homepage_url, employees
        FROM ticker_details
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| TickerDetails {
            ticker: r.ticker.unwrap_or_default(),
            description: r.description,
            homepage_url: r.homepage_url,
            employees: r.employees.map(|e| e.to_string()),
        })
        .collect())
}
