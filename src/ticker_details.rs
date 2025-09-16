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
