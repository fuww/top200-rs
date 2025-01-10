use crate::api;
use crate::config;
use crate::currencies::convert_currency;
use anyhow::Result;
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::SqlitePool;
use std::sync::Arc;

pub async fn marketcaps() -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

    // First fetch exchange rates
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = Arc::new(api::FMPClient::new(api_key));

    println!("Fetching current exchange rates...");
    let exchange_rates = match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            println!("✅ Exchange rates fetched");
            rates
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to fetch exchange rates: {}", e));
        }
    };

    // Create a map of currency pairs to rates
    let mut rate_map = std::collections::HashMap::new();
    for rate in exchange_rates {
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            rate_map.insert(name, price);
        }
    }

    // Get timestamp for data freshness tracking
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Connect to database
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db".to_string());
    let pool = crate::db::create_db_pool(&db_url).await?;

    // Prepare progress bar
    let pb = ProgressBar::new(tickers.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    // Process all tickers and store in database
    for ticker in tickers {
        if let Ok(details) = fmp_client.get_details(&ticker, &rate_map).await {
            let market_cap_eur = details.market_cap.unwrap_or_default();
            let market_cap_usd = convert_currency(market_cap_eur, "EUR", "USD", &rate_map);

            // Prepare values to avoid temporary value drops
            let name = details.name.unwrap_or_default();
            let currency_symbol = details.currency_symbol.unwrap_or_default();
            let active = details.active.unwrap_or_default();
            let description = details.description.unwrap_or_default();
            let homepage_url = details.homepage_url.unwrap_or_default();
            let employees = details.employees.unwrap_or_default().parse::<i64>().ok();

            // Store in database
            sqlx::query!(
                r#"
                INSERT INTO market_caps (
                    ticker, name, market_cap_original, original_currency,
                    market_cap_eur, market_cap_usd, active, description,
                    homepage_url, employees, revenue, revenue_usd,
                    working_capital_ratio, quick_ratio, eps, pe_ratio,
                    de_ratio, roe, timestamp
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(ticker) DO UPDATE SET
                    name = excluded.name,
                    market_cap_original = excluded.market_cap_original,
                    original_currency = excluded.original_currency,
                    market_cap_eur = excluded.market_cap_eur,
                    market_cap_usd = excluded.market_cap_usd,
                    active = excluded.active,
                    description = excluded.description,
                    homepage_url = excluded.homepage_url,
                    employees = excluded.employees,
                    revenue = excluded.revenue,
                    revenue_usd = excluded.revenue_usd,
                    working_capital_ratio = excluded.working_capital_ratio,
                    quick_ratio = excluded.quick_ratio,
                    eps = excluded.eps,
                    pe_ratio = excluded.pe_ratio,
                    de_ratio = excluded.de_ratio,
                    roe = excluded.roe,
                    timestamp = excluded.timestamp,
                    updated_at = CURRENT_TIMESTAMP
                "#,
                details.ticker,
                name,
                details.market_cap,
                currency_symbol,
                market_cap_eur,
                market_cap_usd,
                active,
                description,
                homepage_url,
                employees,
                details.revenue,
                details.revenue_usd,
                details.working_capital_ratio,
                details.quick_ratio,
                details.eps,
                details.pe_ratio,
                details.debt_equity_ratio,
                details.roe,
                timestamp,
            )
            .execute(&pool)
            .await?;
        }
        pb.inc(1);
    }
    pb.finish_with_message("✅ All tickers processed and stored in database");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use sqlx::Row;

    #[tokio::test]
    async fn test_marketcaps_db() -> Result<()> {
        // Set up test database
        let db_url = "sqlite::memory:";
        let pool = db::create_db_pool(db_url).await?;

        // Insert test data
        let test_ticker = "AAPL";
        sqlx::query!(
            r#"
            INSERT INTO market_caps (
                ticker, name, market_cap_original, original_currency,
                market_cap_eur, market_cap_usd, active, description,
                homepage_url, employees, revenue, revenue_usd,
                working_capital_ratio, quick_ratio, eps, pe_ratio,
                de_ratio, roe, timestamp
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            test_ticker,
            "Apple Inc.",
            3000000000000.0,
            "USD",
            2500000000000.0,
            3000000000000.0,
            true,
            "Technology company",
            "https://www.apple.com",
            150000,
            400000000000.0,
            400000000000.0,
            1.5,
            1.2,
            6.5,
            25.0,
            1.8,
            0.45,
            "2025-01-10 14:26:30",
        )
        .execute(&pool)
        .await?;

        // Query and verify data
        let row = sqlx::query("SELECT * FROM market_caps WHERE ticker = ?")
            .bind(test_ticker)
            .fetch_one(&pool)
            .await?;

        assert_eq!(row.get::<String, _>("ticker"), test_ticker);
        assert_eq!(row.get::<String, _>("name"), "Apple Inc.");
        assert_eq!(row.get::<f64, _>("market_cap_eur"), 2500000000000.0);
        assert_eq!(row.get::<f64, _>("market_cap_usd"), 3000000000000.0);
        assert_eq!(row.get::<bool, _>("active"), true);

        // Test ordering by market cap
        let rows = sqlx::query("SELECT ticker, market_cap_eur FROM market_caps ORDER BY market_cap_eur DESC LIMIT 1")
            .fetch_all(&pool)
            .await?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<String, _>("ticker"), test_ticker);

        Ok(())
    }
}
