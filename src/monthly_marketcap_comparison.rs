use crate::config;
use anyhow::Result;
use chrono::Local;
use csv::Writer;
use sqlx::{sqlite::SqlitePool, Row};
use std::path::PathBuf;

pub async fn export_monthly_comparison_csv(pool: &SqlitePool) -> Result<()> {
    let _config = config::load_config()?;

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("monthly_comparison_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Ticker",
        "Company Name",
        "Market Cap Previous Month (EUR)",
        "Market Cap Latest Month (EUR)",
        "Difference (EUR)",
        "Difference (%)",
        "Previous Month Date",
        "Latest Month Date",
        "Warning",
    ])?;

    // Get November and December 2024 data for each ticker
    let records = sqlx::query(
        r#"
        WITH MonthlyData AS (
            SELECT 
                m.ticker,
                m.name,
                CAST(m.market_cap_eur AS REAL) as market_cap_eur,
                date(m.timestamp, 'unixepoch') as date,
                strftime('%Y-%m', datetime(m.timestamp, 'unixepoch')) as month
            FROM market_caps m
            WHERE m.market_cap_eur IS NOT NULL
            AND strftime('%Y-%m', datetime(m.timestamp, 'unixepoch')) IN ('2024-11', '2024-12')
        ),
        RankedData AS (
            SELECT 
                ticker,
                name,
                market_cap_eur,
                date,
                month,
                ROW_NUMBER() OVER (PARTITION BY ticker, month ORDER BY date DESC) as rn
            FROM MonthlyData
        ),
        FilteredData AS (
            SELECT * FROM RankedData WHERE rn = 1
        )
        SELECT 
            d1.ticker,
            d1.name,
            d1.market_cap_eur as latest_market_cap,
            d1.date as latest_date,
            d2.market_cap_eur as previous_market_cap,
            d2.date as previous_date,
            CASE 
                WHEN d2.market_cap_eur IS NULL THEN 'NEW'
                WHEN d1.market_cap_eur < d2.market_cap_eur THEN 'DOWN'
                WHEN d1.market_cap_eur > d2.market_cap_eur THEN 'UP'
                ELSE 'UNCHANGED'
            END as trend
        FROM FilteredData d1
        LEFT JOIN FilteredData d2 
            ON d1.ticker = d2.ticker 
            AND d2.month = '2024-11'
        WHERE d1.month = '2024-12'
        ORDER BY d1.market_cap_eur DESC NULLS LAST
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut total_previous = 0.0;
    let mut total_latest = 0.0;
    let mut count = 0;

    for record in records {
        let ticker: String = record.get("ticker");
        let name: String = record.get("name");
        let latest_market_cap: Option<f64> = record.get("latest_market_cap");
        let latest_date: Option<String> = record.get("latest_date");
        let previous_market_cap: Option<f64> = record.get("previous_market_cap");
        let previous_date: Option<String> = record.get("previous_date");
        let trend: String = record.get("trend");

        let latest_cap = latest_market_cap.unwrap_or_default();
        let previous_cap = previous_market_cap.unwrap_or_default();

        // Update totals for market that existed in both months
        if previous_cap > 0.0 {
            total_previous += previous_cap;
            total_latest += latest_cap;
            count += 1;
        }

        let difference = latest_cap - previous_cap;
        let percentage = if previous_cap > 0.0 {
            (difference / previous_cap) * 100.0
        } else if trend == "NEW" {
            100.0 // New entries show as 100% increase
        } else {
            0.0
        };

        // Add warning for unchanged values
        let warning = if previous_cap > 0.0 && (difference.abs() / previous_cap).abs() < 0.0001 {
            "WARNING: Market cap unchanged between months"
        } else {
            ""
        };

        writer.write_record(&[
            &ticker,
            &name,
            &format!("{:.2}", previous_cap),
            &format!("{:.2}", latest_cap),
            &format!("{:.2}", difference),
            &format!("{:.2}%", percentage),
            &previous_date.unwrap_or_default(),
            &latest_date.unwrap_or_default(),
            warning,
        ])?;
    }

    // Add a summary row
    if count > 0 {
        let total_difference = total_latest - total_previous;
        let total_percentage = (total_difference / total_previous) * 100.0;

        writer.write_record(&[
            "TOTAL",
            "",
            &format!("{:.2}", total_previous),
            &format!("{:.2}", total_latest),
            &format!("{:.2}", total_difference),
            &format!("{:.2}%", total_percentage),
            "",
            "",
            "",
        ])?;
    }

    println!(
        "\n✅ Monthly comparison CSV file created at: {}",
        csv_path.display()
    );
    Ok(())
}
