use anyhow::Result;
use chrono::Local;
use csv::Writer;
use sqlx::{Row, SqlitePool};

pub async fn generate_reports(pool: &SqlitePool) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    
    // Get the latest market caps from the database
    let market_caps = sqlx::query(
        r#"
        WITH latest_market_caps AS (
            SELECT ticker,
                   MAX(timestamp) as max_timestamp
            FROM market_caps
            GROUP BY ticker
        )
        SELECT 
            m.ticker,
            m.name,
            CAST(m.market_cap_original AS REAL) as market_cap_original,
            m.original_currency,
            CAST(m.market_cap_eur AS REAL) as market_cap_eur,
            CAST(m.market_cap_usd AS REAL) as market_cap_usd,
            m.exchange,
            CAST(m.price AS REAL) as price,
            m.active,
            m.description,
            m.homepage_url,
            m.employees,
            CAST(m.revenue AS REAL) as revenue,
            CAST(m.revenue_usd AS REAL) as revenue_usd,
            CAST(m.working_capital_ratio AS REAL) as working_capital_ratio,
            CAST(m.quick_ratio AS REAL) as quick_ratio,
            CAST(m.eps AS REAL) as eps,
            CAST(m.pe_ratio AS REAL) as pe_ratio,
            CAST(m.de_ratio AS REAL) as de_ratio,
            CAST(m.roe AS REAL) as roe,
            m.timestamp
        FROM market_caps m
        INNER JOIN latest_market_caps lm
            ON m.ticker = lm.ticker
            AND m.timestamp = lm.max_timestamp
        ORDER BY m.market_cap_eur DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    // Generate combined market caps CSV
    let filename = format!("output/combined_marketcaps_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Symbol",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
        "Timestamp",
    ])?;

    // Write all results
    for record in &market_caps {
        writer.write_record(&[
            &record.get::<String, _>("ticker"),
            &record.get::<String, _>("name"),
            &record.get::<Option<f64>, _>("market_cap_original").unwrap_or_default().to_string(),
            &record.get::<Option<String>, _>("original_currency").unwrap_or_default(),
            &record.get::<Option<f64>, _>("market_cap_eur").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("market_cap_usd").unwrap_or_default().to_string(),
            &record.get::<Option<String>, _>("exchange").unwrap_or_default(),
            &record.get::<Option<f64>, _>("price").unwrap_or_default().to_string(),
            &record.get::<Option<bool>, _>("active").unwrap_or_default().to_string(),
            &record.get::<Option<String>, _>("description").unwrap_or_default(),
            &record.get::<Option<String>, _>("homepage_url").unwrap_or_default(),
            &record.get::<Option<i32>, _>("employees").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("revenue").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("revenue_usd").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("working_capital_ratio").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("quick_ratio").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("eps").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("pe_ratio").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("de_ratio").unwrap_or_default().to_string(),
            &record.get::<Option<f64>, _>("roe").unwrap_or_default().to_string(),
            &record.get::<i64, _>("timestamp").to_string(),
        ])?;
    }
    writer.flush()?;
    println!("✅ Combined market caps written to: {}", filename);

    // Generate top 100 active companies CSV
    let top_100_filename = format!("output/top_100_active_{}.csv", timestamp);
    let top_100_file = std::fs::File::create(&top_100_filename)?;
    let mut top_100_writer = Writer::from_writer(top_100_file);

    // Write headers (same as above)
    top_100_writer.write_record(&[
        "Symbol",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
        "Timestamp",
    ])?;

    // Write top 100 active companies
    let mut count = 0;
    for record in &market_caps {
        if record.get::<Option<bool>, _>("active").unwrap_or_default() && count < 100 {
            top_100_writer.write_record(&[
                &record.get::<String, _>("ticker"),
                &record.get::<String, _>("name"),
                &record.get::<Option<f64>, _>("market_cap_original").unwrap_or_default().to_string(),
                &record.get::<Option<String>, _>("original_currency").unwrap_or_default(),
                &record.get::<Option<f64>, _>("market_cap_eur").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("market_cap_usd").unwrap_or_default().to_string(),
                &record.get::<Option<String>, _>("exchange").unwrap_or_default(),
                &record.get::<Option<f64>, _>("price").unwrap_or_default().to_string(),
                &record.get::<Option<bool>, _>("active").unwrap_or_default().to_string(),
                &record.get::<Option<String>, _>("description").unwrap_or_default(),
                &record.get::<Option<String>, _>("homepage_url").unwrap_or_default(),
                &record.get::<Option<i32>, _>("employees").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("revenue").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("revenue_usd").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("working_capital_ratio").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("quick_ratio").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("eps").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("pe_ratio").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("de_ratio").unwrap_or_default().to_string(),
                &record.get::<Option<f64>, _>("roe").unwrap_or_default().to_string(),
                &record.get::<i64, _>("timestamp").to_string(),
            ])?;
            count += 1;
        }
    }
    top_100_writer.flush()?;
    println!("✅ Top 100 active tickers written to: {}", top_100_filename);

    Ok(())
}
