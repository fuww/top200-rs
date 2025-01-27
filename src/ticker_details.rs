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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn test_ticker_details_operations() -> Result<()> {
        // Set up test database
        let pool = db::create_test_pool().await?;
        db::migrate(&pool).await?;

        // Test inserting ticker details
        let details = TickerDetails {
            ticker: "TEST".to_string(),
            description: Some("Test Company".to_string()),
            homepage_url: Some("https://test.com".to_string()),
            employees: Some("1000".to_string()),
        };
        update_ticker_details(&pool, &details).await?;

        // Test getting ticker details
        let fetched = get_ticker_details(&pool, "TEST").await?.unwrap();
        assert_eq!(fetched.ticker, "TEST");
        assert_eq!(fetched.description.unwrap(), "Test Company");
        assert_eq!(fetched.homepage_url.unwrap(), "https://test.com");
        assert_eq!(fetched.employees.unwrap(), "1000");

        // Test updating ticker details
        let updated_details = TickerDetails {
            ticker: "TEST".to_string(),
            description: Some("Updated Description".to_string()),
            homepage_url: Some("https://updated.com".to_string()),
            employees: Some("2000".to_string()),
        };
        update_ticker_details(&pool, &updated_details).await?;

        // Verify update
        let fetched = get_ticker_details(&pool, "TEST").await?.unwrap();
        assert_eq!(fetched.description.unwrap(), "Updated Description");
        assert_eq!(fetched.homepage_url.unwrap(), "https://updated.com");
        assert_eq!(fetched.employees.unwrap(), "2000");

        // Test getting non-existent ticker
        let not_found = get_ticker_details(&pool, "NOTFOUND").await?;
        assert!(not_found.is_none());

        // Test listing all ticker details
        let details2 = TickerDetails {
            ticker: "TEST2".to_string(),
            description: None,
            homepage_url: None,
            employees: None,
        };
        update_ticker_details(&pool, &details2).await?;

        let all_details = list_ticker_details(&pool).await?;
        assert_eq!(all_details.len(), 2);
        assert!(all_details.iter().any(|d| d.ticker == "TEST"));
        assert!(all_details.iter().any(|d| d.ticker == "TEST2"));

        Ok(())
    }
}
