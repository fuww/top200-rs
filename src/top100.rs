// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sqlx::SqlitePool;
use std::fs::File;
use std::io::Write;
use chrono::Utc;

/// Represents a company in the top 100 list
#[derive(Debug)]
pub struct Top100Company {
    pub rank: i32,
    pub ticker: String,
    pub name: String,
    pub market_cap_usd: f64,
}

/// Get the top 100 companies by market cap in USD from the database
pub async fn get_top_100(pool: &SqlitePool) -> Result<Vec<Top100Company>> {
    let records = sqlx::query!(
        r#"
        SELECT 
            ticker,
            name,
            market_cap_usd
        FROM market_caps
        WHERE active = true
        AND market_cap_usd IS NOT NULL
        ORDER BY market_cap_usd DESC
        LIMIT 100
        "#
    )
    .fetch_all(pool)
    .await?;

    let companies: Vec<Top100Company> = records
        .into_iter()
        .enumerate()
        .filter_map(|(i, r)| {
            // Only include records with valid ticker and market cap
            let ticker = r.ticker?;
            let market_cap_usd = r.market_cap_usd?;
            
            Some(Top100Company {
                rank: (i + 1) as i32,
                ticker,
                name: r.name,
                market_cap_usd,
            })
        })
        .collect();

    Ok(companies)
}

/// Write the top 100 companies to a CSV file
pub fn write_to_csv(companies: &[Top100Company], output_path: Option<String>) -> Result<String> {
    // Generate default filename if none provided
    let filename = output_path.unwrap_or_else(|| {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("top100_marketcap_{}.csv", timestamp)
    });

    let mut file = File::create(&filename)?;

    // Write header
    writeln!(file, "Rank,Ticker,Name,Market Cap USD")?;

    // Write data
    for company in companies {
        writeln!(
            file,
            "{},{},\"{}\",{:.2}",
            company.rank,
            company.ticker,
            company.name.replace('"', "\"\""), // Escape quotes for CSV
            company.market_cap_usd
        )?;
    }

    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::fs;

    #[tokio::test]
    async fn test_top_100() -> Result<()> {
        // Set up test database
        let db_url = "sqlite::memory:";
        let pool = db::create_db_pool(db_url).await?;
        sqlx::migrate!().run(&pool).await?;

        // Insert test data
        sqlx::query!(
            r#"
            INSERT INTO market_caps (
                ticker, name, market_cap_usd, active
            ) VALUES 
            ('AAPL', 'Apple Inc.', 3000000000000.0, true),
            ('MSFT', 'Microsoft Corporation', 2800000000000.0, true),
            ('GOOG', 'Alphabet Inc.', 2000000000000.0, true),
            ('AMZN', 'Amazon.com Inc.', 1500000000000.0, true),
            ('INACTIVE', 'Inactive Company', 9999999999999.0, false)
            "#
        )
        .execute(&pool)
        .await?;

        // Get top 100
        let companies = get_top_100(&pool).await?;

        // Verify results
        assert_eq!(companies.len(), 4);
        assert_eq!(companies[0].ticker, "AAPL");
        assert_eq!(companies[0].rank, 1);
        assert_eq!(companies[1].ticker, "MSFT");
        assert_eq!(companies[1].rank, 2);
        assert_eq!(companies[2].ticker, "GOOG");
        assert_eq!(companies[2].rank, 3);
        assert_eq!(companies[3].ticker, "AMZN");
        assert_eq!(companies[3].rank, 4);

        // Test CSV writing
        let test_file = "test_top100.csv";
        write_to_csv(&companies, Some(test_file.to_string()))?;

        // Verify CSV contents
        let csv_contents = fs::read_to_string(test_file)?;
        let lines: Vec<&str> = csv_contents.lines().collect();
        assert_eq!(lines.len(), 5); // Header + 4 companies
        assert!(lines[0].contains("Rank,Ticker,Name,Market Cap USD"));
        assert!(lines[1].contains("AAPL"));
        assert!(lines[2].contains("MSFT"));

        // Clean up test file
        fs::remove_file(test_file)?;

        Ok(())
    }
}
