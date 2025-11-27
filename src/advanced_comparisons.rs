// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Advanced comparison features for market cap analysis
//!
//! This module provides:
//! - Multi-date trend analysis (more than 2 dates)
//! - Year-over-Year (YoY) comparisons
//! - Quarter-over-Quarter (QoQ) comparisons
//! - Rolling period comparisons (30d, 90d, 1y)
//! - Benchmark comparisons (S&P 500, MSCI indices)
//! - Peer group comparisons

use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use csv::{Reader, Writer};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::Path;

use crate::currencies::{convert_currency, get_rate_map_from_db_for_date};

/// Market cap record from CSV file
#[derive(Debug, Deserialize, Clone)]
pub struct MarketCapRecord {
    #[serde(rename = "Rank")]
    pub rank: Option<usize>,
    #[serde(rename = "Ticker")]
    pub ticker: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Market Cap (Original)")]
    pub market_cap_original: Option<f64>,
    #[serde(rename = "Original Currency")]
    pub original_currency: Option<String>,
    #[serde(rename = "Market Cap (EUR)")]
    pub market_cap_eur: Option<f64>,
    #[serde(rename = "Market Cap (USD)")]
    pub market_cap_usd: Option<f64>,
}

/// Data point for trend analysis
#[derive(Debug, Clone, Serialize)]
pub struct TrendDataPoint {
    pub date: String,
    pub market_cap_usd: Option<f64>,
    pub rank: Option<usize>,
    pub market_share: Option<f64>,
}

/// Trend analysis result for a single ticker
#[derive(Debug, Clone, Serialize)]
pub struct TickerTrend {
    pub ticker: String,
    pub name: String,
    pub data_points: Vec<TrendDataPoint>,
    pub overall_change_pct: Option<f64>,
    pub overall_change_abs: Option<f64>,
    pub cagr: Option<f64>, // Compound Annual Growth Rate
    pub volatility: Option<f64>,
    pub max_drawdown: Option<f64>,
}

/// Summary statistics for multi-date analysis
#[derive(Debug, Clone, Serialize)]
pub struct TrendSummary {
    pub start_date: String,
    pub end_date: String,
    pub num_periods: usize,
    pub total_market_cap_start: f64,
    pub total_market_cap_end: f64,
    pub total_change_pct: f64,
    pub best_performer: Option<(String, f64)>,
    pub worst_performer: Option<(String, f64)>,
    pub most_volatile: Option<(String, f64)>,
    pub most_stable: Option<(String, f64)>,
}

/// Rolling period configuration
#[derive(Debug, Clone, Copy)]
pub enum RollingPeriod {
    Days30,
    Days90,
    Days180,
    Year1,
    Custom(i64),
}

impl RollingPeriod {
    pub fn days(&self) -> i64 {
        match self {
            RollingPeriod::Days30 => 30,
            RollingPeriod::Days90 => 90,
            RollingPeriod::Days180 => 180,
            RollingPeriod::Year1 => 365,
            RollingPeriod::Custom(d) => *d,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RollingPeriod::Days30 => "30-day",
            RollingPeriod::Days90 => "90-day",
            RollingPeriod::Days180 => "180-day",
            RollingPeriod::Year1 => "1-year",
            RollingPeriod::Custom(_) => "custom",
        }
    }
}

/// Benchmark types for comparison
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Benchmark {
    SP500,
    MSCI,
    Custom(String),
}

impl Benchmark {
    pub fn name(&self) -> &str {
        match self {
            Benchmark::SP500 => "S&P 500",
            Benchmark::MSCI => "MSCI World",
            Benchmark::Custom(name) => name,
        }
    }

    pub fn ticker(&self) -> &str {
        match self {
            Benchmark::SP500 => "SPY", // S&P 500 ETF proxy
            Benchmark::MSCI => "URTH", // MSCI World ETF proxy
            Benchmark::Custom(ticker) => ticker,
        }
    }
}

/// Peer group definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeerGroup {
    pub name: String,
    pub description: Option<String>,
    pub tickers: Vec<String>,
}

impl Default for PeerGroup {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            tickers: Vec::new(),
        }
    }
}

/// Predefined peer groups for the fashion/retail industry
pub fn get_predefined_peer_groups() -> Vec<PeerGroup> {
    vec![
        PeerGroup {
            name: "Luxury".to_string(),
            description: Some("High-end luxury fashion houses".to_string()),
            tickers: vec![
                "MC.PA".to_string(),   // LVMH
                "RMS.PA".to_string(),  // Hermès
                "KER.PA".to_string(),  // Kering
                "CDI.PA".to_string(),  // Dior
                "CFR.SW".to_string(),  // Richemont
                "MONC.MI".to_string(), // Moncler
                "BC.MI".to_string(),   // Brunello Cucinelli
                "BRBY.L".to_string(),  // Burberry
                "1913.HK".to_string(), // Prada
                "ZGN".to_string(),     // Zegna
            ],
        },
        PeerGroup {
            name: "Sportswear".to_string(),
            description: Some("Athletic and sportswear brands".to_string()),
            tickers: vec![
                "NKE".to_string(),    // Nike
                "ADS.DE".to_string(), // Adidas
                "PUM.DE".to_string(), // Puma
                "LULU".to_string(),   // Lululemon
                "UA".to_string(),     // Under Armour
                "ONON".to_string(),   // On Holding
                "DECK".to_string(),   // Deckers
                "SKX".to_string(),    // Skechers
                "COLM".to_string(),   // Columbia
                "7936.T".to_string(), // Asics
            ],
        },
        PeerGroup {
            name: "Fast Fashion".to_string(),
            description: Some("Fast fashion and value retailers".to_string()),
            tickers: vec![
                "ITX.MC".to_string(),  // Inditex
                "HM-B.ST".to_string(), // H&M
                "9983.T".to_string(),  // Fast Retailing
                "GAP".to_string(),     // Gap
                "ANF".to_string(),     // Abercrombie & Fitch
                "URBN".to_string(),    // Urban Outfitters
                "AEO".to_string(),     // American Eagle
                "GES".to_string(),     // Guess
                "LPP.WA".to_string(),  // LPP
                "BOO.L".to_string(),   // Boohoo
            ],
        },
        PeerGroup {
            name: "Department Stores".to_string(),
            description: Some("Multi-brand department store retailers".to_string()),
            tickers: vec![
                "M".to_string(),     // Macy's
                "JWN".to_string(),   // Nordstrom
                "KSS".to_string(),   // Kohl's
                "DDS".to_string(),   // Dillard's
                "NXT.L".to_string(), // Next
                "MKS.L".to_string(), // Marks & Spencer
                "JD.L".to_string(),  // JD Sports
            ],
        },
        PeerGroup {
            name: "Value Retail".to_string(),
            description: Some("Off-price and discount retailers".to_string()),
            tickers: vec![
                "TJX".to_string(),  // TJX Companies
                "ROST".to_string(), // Ross Stores
                "BURL".to_string(), // Burlington
                "FL".to_string(),   // Foot Locker
                "SVV".to_string(),  // Savers Value Village
            ],
        },
        PeerGroup {
            name: "Footwear".to_string(),
            description: Some("Footwear-focused brands".to_string()),
            tickers: vec![
                "NKE".to_string(),     // Nike
                "BIRK".to_string(),    // Birkenstock
                "CROX".to_string(),    // Crocs
                "DECK".to_string(),    // Deckers
                "SKX".to_string(),     // Skechers
                "WWW".to_string(),     // Wolverine
                "SHOO".to_string(),    // Steve Madden
                "CAL".to_string(),     // Caleres
                "BOOT".to_string(),    // Boot Barn
                "GCO".to_string(),     // Genesco
                "SFER.MI".to_string(), // Ferragamo
                "TOD.MI".to_string(),  // Tod's
            ],
        },
        PeerGroup {
            name: "E-commerce".to_string(),
            description: Some("Online-first fashion retailers".to_string()),
            tickers: vec![
                "ZAL.DE".to_string(),   // Zalando
                "VIPS".to_string(),     // Vipshop
                "RVLV".to_string(),     // Revolve
                "TDUP".to_string(),     // ThredUp
                "REAL".to_string(),     // The RealReal
                "RENT".to_string(),     // Rent the Runway
                "LUXE".to_string(),     // Mytheresa
                "BOOZT.ST".to_string(), // Boozt
                "YOU.DE".to_string(),   // About You
                "BOO.L".to_string(),    // Boohoo
            ],
        },
        PeerGroup {
            name: "Asian Fashion".to_string(),
            description: Some("Major Asian fashion companies".to_string()),
            tickers: vec![
                "9983.T".to_string(),  // Fast Retailing
                "1929.HK".to_string(), // Chow Tai Fook
                "1913.HK".to_string(), // Prada
                "2331.HK".to_string(), // Li Ning
                "3998.HK".to_string(), // Bosideng
                "6110.HK".to_string(), // Topsports
                "7936.T".to_string(),  // Asics
                "7606.T".to_string(),  // United Arrows
            ],
        },
    ]
}

/// Find the most recent CSV file for a given date
pub fn find_csv_for_date(date: &str) -> Result<String> {
    let output_dir = Path::new("output");
    let pattern = format!("marketcaps_{}_", date);

    let mut matching_files = Vec::new();
    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if file_name_str.starts_with(&pattern) && file_name_str.ends_with(".csv") {
            matching_files.push(file_name_str.to_string());
        }
    }

    if matching_files.is_empty() {
        anyhow::bail!(
            "No CSV file found for date {}. Please run 'fetch-specific-date-market-caps {}' first.",
            date,
            date
        );
    }

    // Sort to get the most recent file (by filename timestamp)
    matching_files.sort();
    let selected_file = matching_files.last().unwrap();

    Ok(format!("output/{}", selected_file))
}

/// Read market cap data from CSV file
pub fn read_market_cap_csv(file_path: &str) -> Result<Vec<MarketCapRecord>> {
    let file =
        File::open(file_path).with_context(|| format!("Failed to open CSV file: {}", file_path))?;

    let mut reader = Reader::from_reader(file);
    let mut records = Vec::new();

    for result in reader.deserialize() {
        let record: MarketCapRecord = result?;
        records.push(record);
    }

    Ok(records)
}

/// Calculate market shares for records
fn calculate_market_shares(records: &[MarketCapRecord]) -> HashMap<String, f64> {
    let total_market_cap: f64 = records.iter().filter_map(|r| r.market_cap_usd).sum();

    let mut shares = HashMap::new();
    if total_market_cap > 0.0 {
        for record in records {
            if let Some(market_cap) = record.market_cap_usd {
                let share = (market_cap / total_market_cap) * 100.0;
                shares.insert(record.ticker.clone(), share);
            }
        }
    }
    shares
}

/// Get available dates from the output directory
pub fn get_available_dates() -> Result<Vec<String>> {
    let output_dir = Path::new("output");
    let mut dates = HashSet::new();

    // Return empty list if output directory doesn't exist
    if !output_dir.exists() {
        println!("Output directory does not exist. No market cap data available.");
        println!(
            "Run 'cargo run -- fetch-specific-date-market-caps YYYY-MM-DD' to fetch data first."
        );
        return Ok(Vec::new());
    }

    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if file_name_str.starts_with("marketcaps_") && file_name_str.ends_with(".csv") {
            // Extract date from filename: marketcaps_YYYY-MM-DD_...
            if let Some(date_part) = file_name_str.strip_prefix("marketcaps_") {
                if let Some(date) = date_part.split('_').next() {
                    if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok() {
                        dates.insert(date.to_string());
                    }
                }
            }
        }
    }

    let mut sorted_dates: Vec<String> = dates.into_iter().collect();
    sorted_dates.sort();
    Ok(sorted_dates)
}

// =====================================================
// Multi-date Trend Analysis
// =====================================================

/// Perform multi-date trend analysis
pub async fn analyze_trends(
    pool: &SqlitePool,
    dates: Vec<String>,
) -> Result<(Vec<TickerTrend>, TrendSummary)> {
    if dates.len() < 2 {
        anyhow::bail!("At least 2 dates are required for trend analysis");
    }

    println!(
        "Analyzing trends across {} dates: {} to {}",
        dates.len(),
        dates.first().unwrap(),
        dates.last().unwrap()
    );

    let progress = ProgressBar::new(dates.len() as u64 + 2);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Get exchange rates for normalization (use the latest date)
    let latest_date = dates.last().unwrap();
    let latest_date_parsed = NaiveDate::parse_from_str(latest_date, "%Y-%m-%d")?;
    let latest_timestamp = NaiveDateTime::new(latest_date_parsed, NaiveTime::default())
        .and_utc()
        .timestamp();

    progress.set_message("Loading exchange rates...");
    let normalization_rates = get_rate_map_from_db_for_date(pool, Some(latest_timestamp)).await?;
    progress.inc(1);

    // Load data for each date
    let mut all_data: BTreeMap<String, HashMap<String, MarketCapRecord>> = BTreeMap::new();
    let mut all_tickers: HashSet<String> = HashSet::new();
    let mut ticker_names: HashMap<String, String> = HashMap::new();

    for date in &dates {
        progress.set_message(format!("Loading data for {}...", date));
        let file_path = find_csv_for_date(date)?;
        let records = read_market_cap_csv(&file_path)?;

        let mut date_map = HashMap::new();
        for record in records {
            all_tickers.insert(record.ticker.clone());
            ticker_names.insert(record.ticker.clone(), record.name.clone());
            date_map.insert(record.ticker.clone(), record);
        }
        all_data.insert(date.clone(), date_map);
        progress.inc(1);
    }

    progress.set_message("Calculating trends...");

    // Build trend data for each ticker
    let mut trends: Vec<TickerTrend> = Vec::new();

    for ticker in &all_tickers {
        let name = ticker_names.get(ticker).cloned().unwrap_or_default();
        let mut data_points = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        for date in &dates {
            if let Some(date_data) = all_data.get(date) {
                if let Some(record) = date_data.get(ticker) {
                    // Normalize market cap using latest exchange rates
                    let market_cap_usd = record.market_cap_original.map(|orig| {
                        let currency = record.original_currency.as_deref().unwrap_or("USD");
                        if normalization_rates.is_empty() {
                            record.market_cap_usd.unwrap_or(orig)
                        } else {
                            convert_currency(orig, currency, "USD", &normalization_rates)
                        }
                    });

                    let shares =
                        calculate_market_shares(&date_data.values().cloned().collect::<Vec<_>>());

                    data_points.push(TrendDataPoint {
                        date: date.clone(),
                        market_cap_usd,
                        rank: record.rank,
                        market_share: shares.get(ticker).copied(),
                    });

                    if let Some(v) = market_cap_usd {
                        values.push(v);
                    }
                } else {
                    // Ticker not present on this date
                    data_points.push(TrendDataPoint {
                        date: date.clone(),
                        market_cap_usd: None,
                        rank: None,
                        market_share: None,
                    });
                }
            }
        }

        // Calculate statistics
        let overall_change_pct = if values.len() >= 2 {
            let first = values.first().unwrap();
            let last = values.last().unwrap();
            if *first > 0.0 {
                Some(((last - first) / first) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        let overall_change_abs = if values.len() >= 2 {
            let first = values.first().unwrap();
            let last = values.last().unwrap();
            Some(last - first)
        } else {
            None
        };

        // Calculate CAGR (Compound Annual Growth Rate)
        let cagr = if values.len() >= 2 && dates.len() >= 2 {
            let first = *values.first().unwrap();
            let last = *values.last().unwrap();
            let first_date = NaiveDate::parse_from_str(dates.first().unwrap(), "%Y-%m-%d")?;
            let last_date = NaiveDate::parse_from_str(dates.last().unwrap(), "%Y-%m-%d")?;
            let years = (last_date - first_date).num_days() as f64 / 365.25;
            if first > 0.0 && years > 0.0 {
                Some(((last / first).powf(1.0 / years) - 1.0) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        // Calculate volatility (standard deviation of returns)
        let volatility = if values.len() >= 3 {
            let returns: Vec<f64> = values
                .windows(2)
                .map(|w| (w[1] - w[0]) / w[0] * 100.0)
                .collect();
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance =
                returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
            Some(variance.sqrt())
        } else {
            None
        };

        // Calculate max drawdown
        let max_drawdown = if values.len() >= 2 {
            let mut max_so_far = values[0];
            let mut max_dd = 0.0f64;
            for &v in &values {
                if v > max_so_far {
                    max_so_far = v;
                }
                let dd = (max_so_far - v) / max_so_far * 100.0;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
            Some(max_dd)
        } else {
            None
        };

        trends.push(TickerTrend {
            ticker: ticker.clone(),
            name,
            data_points,
            overall_change_pct,
            overall_change_abs,
            cagr,
            volatility,
            max_drawdown,
        });
    }

    // Sort by overall change percentage
    trends.sort_by(|a, b| {
        let a_pct = a.overall_change_pct.unwrap_or(f64::NEG_INFINITY);
        let b_pct = b.overall_change_pct.unwrap_or(f64::NEG_INFINITY);
        b_pct.partial_cmp(&a_pct).unwrap()
    });

    // Calculate summary statistics
    let total_start: f64 = trends
        .iter()
        .filter_map(|t| t.data_points.first().and_then(|dp| dp.market_cap_usd))
        .sum();
    let total_end: f64 = trends
        .iter()
        .filter_map(|t| t.data_points.last().and_then(|dp| dp.market_cap_usd))
        .sum();

    let best_performer = trends
        .iter()
        .filter_map(|t| t.overall_change_pct.map(|p| (t.ticker.clone(), p)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let worst_performer = trends
        .iter()
        .filter_map(|t| t.overall_change_pct.map(|p| (t.ticker.clone(), p)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let most_volatile = trends
        .iter()
        .filter_map(|t| t.volatility.map(|v| (t.ticker.clone(), v)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let most_stable = trends
        .iter()
        .filter_map(|t| t.volatility.map(|v| (t.ticker.clone(), v)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let summary = TrendSummary {
        start_date: dates.first().unwrap().clone(),
        end_date: dates.last().unwrap().clone(),
        num_periods: dates.len(),
        total_market_cap_start: total_start,
        total_market_cap_end: total_end,
        total_change_pct: if total_start > 0.0 {
            ((total_end - total_start) / total_start) * 100.0
        } else {
            0.0
        },
        best_performer,
        worst_performer,
        most_volatile,
        most_stable,
    };

    progress.inc(1);
    progress.finish_with_message("Trend analysis complete");

    Ok((trends, summary))
}

/// Export trend analysis results
pub fn export_trend_analysis(
    trends: &[TickerTrend],
    summary: &TrendSummary,
    dates: &[String],
) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_filename = format!(
        "output/trend_analysis_{}_to_{}_{}.csv",
        summary.start_date, summary.end_date, timestamp
    );
    let md_filename = format!(
        "output/trend_analysis_{}_to_{}_summary_{}.md",
        summary.start_date, summary.end_date, timestamp
    );

    // Export CSV
    let file = File::create(&csv_filename)?;
    let mut writer = Writer::from_writer(file);

    // Build headers with date columns
    let mut headers = vec![
        "Ticker".to_string(),
        "Name".to_string(),
        "Overall Change (%)".to_string(),
        "Overall Change ($)".to_string(),
        "CAGR (%)".to_string(),
        "Volatility".to_string(),
        "Max Drawdown (%)".to_string(),
    ];
    for date in dates {
        headers.push(format!("Market Cap {}", date));
        headers.push(format!("Rank {}", date));
    }
    writer.write_record(&headers)?;

    // Write data rows
    for trend in trends {
        let mut row = vec![
            trend.ticker.clone(),
            trend.name.clone(),
            trend
                .overall_change_pct
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            trend
                .overall_change_abs
                .map(|v| format!("{:.0}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            trend
                .cagr
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            trend
                .volatility
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            trend
                .max_drawdown
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
        ];

        for date in dates {
            let dp = trend.data_points.iter().find(|dp| &dp.date == date);
            row.push(
                dp.and_then(|d| d.market_cap_usd)
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "N/A".to_string()),
            );
            row.push(
                dp.and_then(|d| d.rank)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            );
        }
        writer.write_record(&row)?;
    }
    writer.flush()?;
    println!("Trend data exported to {}", csv_filename);

    // Export Markdown summary
    let mut file = File::create(&md_filename)?;

    writeln!(
        file,
        "# Trend Analysis: {} to {}",
        summary.start_date, summary.end_date
    )?;
    writeln!(file)?;
    writeln!(file, "## Overview")?;
    writeln!(
        file,
        "- **Period**: {} to {}",
        summary.start_date, summary.end_date
    )?;
    writeln!(file, "- **Data Points**: {} dates", summary.num_periods)?;
    writeln!(
        file,
        "- **Total Market Cap (Start)**: ${:.2}B",
        summary.total_market_cap_start / 1_000_000_000.0
    )?;
    writeln!(
        file,
        "- **Total Market Cap (End)**: ${:.2}B",
        summary.total_market_cap_end / 1_000_000_000.0
    )?;
    writeln!(file, "- **Total Change**: {:.2}%", summary.total_change_pct)?;
    writeln!(file)?;

    writeln!(file, "## Key Performers")?;
    if let Some((ticker, pct)) = &summary.best_performer {
        writeln!(file, "- **Best Performer**: {} (+{:.2}%)", ticker, pct)?;
    }
    if let Some((ticker, pct)) = &summary.worst_performer {
        writeln!(file, "- **Worst Performer**: {} ({:.2}%)", ticker, pct)?;
    }
    if let Some((ticker, vol)) = &summary.most_volatile {
        writeln!(
            file,
            "- **Most Volatile**: {} (volatility: {:.2})",
            ticker, vol
        )?;
    }
    if let Some((ticker, vol)) = &summary.most_stable {
        writeln!(
            file,
            "- **Most Stable**: {} (volatility: {:.2})",
            ticker, vol
        )?;
    }
    writeln!(file)?;

    writeln!(file, "## Top 10 Performers")?;
    writeln!(file, "| Rank | Ticker | Name | Change (%) | CAGR (%) |")?;
    writeln!(file, "|------|--------|------|------------|----------|")?;
    for (i, trend) in trends.iter().take(10).enumerate() {
        writeln!(
            file,
            "| {} | [{}](https://finance.yahoo.com/quote/{}/) | {} | {:.2}% | {}% |",
            i + 1,
            trend.ticker,
            trend.ticker,
            trend.name,
            trend.overall_change_pct.unwrap_or(0.0),
            trend
                .cagr
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string())
        )?;
    }
    writeln!(file)?;

    writeln!(file, "## Bottom 10 Performers")?;
    writeln!(file, "| Rank | Ticker | Name | Change (%) | CAGR (%) |")?;
    writeln!(file, "|------|--------|------|------------|----------|")?;
    let bottom_10: Vec<_> = trends.iter().rev().take(10).collect();
    for (i, trend) in bottom_10.iter().enumerate() {
        writeln!(
            file,
            "| {} | [{}](https://finance.yahoo.com/quote/{}/) | {} | {:.2}% | {}% |",
            i + 1,
            trend.ticker,
            trend.ticker,
            trend.name,
            trend.overall_change_pct.unwrap_or(0.0),
            trend
                .cagr
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string())
        )?;
    }
    writeln!(file)?;

    writeln!(file, "---")?;
    writeln!(
        file,
        "*Generated on {}*",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    println!("Summary report exported to {}", md_filename);

    Ok(())
}

// =====================================================
// Year-over-Year (YoY) Comparison
// =====================================================

/// Calculate dates for YoY comparison
pub fn get_yoy_dates(reference_date: &str, num_years: i32) -> Result<Vec<String>> {
    let ref_date = NaiveDate::parse_from_str(reference_date, "%Y-%m-%d")
        .context("Invalid date format. Use YYYY-MM-DD")?;

    let mut dates = Vec::new();
    for i in 0..=num_years {
        let year = ref_date.year() - i;
        // Handle Feb 29 on non-leap years
        let date = if ref_date.month() == 2 && ref_date.day() == 29 {
            NaiveDate::from_ymd_opt(year, 2, 28)
        } else {
            NaiveDate::from_ymd_opt(year, ref_date.month(), ref_date.day())
        };
        if let Some(d) = date {
            dates.push(d.format("%Y-%m-%d").to_string());
        }
    }

    dates.reverse(); // Oldest first
    Ok(dates)
}

/// Perform YoY comparison
pub async fn compare_yoy(pool: &SqlitePool, reference_date: &str, num_years: i32) -> Result<()> {
    println!(
        "Performing Year-over-Year comparison for {} ({} years back)",
        reference_date, num_years
    );

    let dates = get_yoy_dates(reference_date, num_years)?;
    let available_dates = get_available_dates()?;

    // Filter to only available dates
    let valid_dates: Vec<String> = dates
        .into_iter()
        .filter(|d| available_dates.contains(d))
        .collect();

    if valid_dates.len() < 2 {
        anyhow::bail!(
            "Not enough data for YoY comparison. Found {} dates, need at least 2.\n\
            Available dates: {:?}\n\
            Requested dates need to be fetched first using 'fetch-specific-date-market-caps'",
            valid_dates.len(),
            available_dates
        );
    }

    println!("Using {} dates for YoY analysis:", valid_dates.len());
    for date in &valid_dates {
        println!("  - {}", date);
    }

    let (trends, summary) = analyze_trends(pool, valid_dates.clone()).await?;
    export_trend_analysis(&trends, &summary, &valid_dates)?;

    Ok(())
}

// =====================================================
// Quarter-over-Quarter (QoQ) Comparison
// =====================================================

/// Calculate dates for QoQ comparison (end of each quarter)
pub fn get_qoq_dates(reference_date: &str, num_quarters: i32) -> Result<Vec<String>> {
    let ref_date = NaiveDate::parse_from_str(reference_date, "%Y-%m-%d")
        .context("Invalid date format. Use YYYY-MM-DD")?;

    let mut dates = Vec::new();

    for i in 0..=num_quarters {
        let months_back = i * 3;
        let target_date = ref_date - Duration::days(months_back as i64 * 30);

        // Find quarter end
        let quarter_end = get_quarter_end(target_date)?;
        dates.push(quarter_end.format("%Y-%m-%d").to_string());
    }

    dates.reverse(); // Oldest first
    dates.dedup(); // Remove duplicates
    Ok(dates)
}

/// Get the end date of the quarter containing the given date
fn get_quarter_end(date: NaiveDate) -> Result<NaiveDate> {
    let month = date.month();
    let year = date.year();

    let (end_month, end_day) = match month {
        1..=3 => (3, 31),
        4..=6 => (6, 30),
        7..=9 => (9, 30),
        10..=12 => (12, 31),
        _ => unreachable!(),
    };

    NaiveDate::from_ymd_opt(year, end_month, end_day)
        .context("Failed to construct quarter end date")
}

/// Perform QoQ comparison
pub async fn compare_qoq(pool: &SqlitePool, reference_date: &str, num_quarters: i32) -> Result<()> {
    println!(
        "Performing Quarter-over-Quarter comparison for {} ({} quarters back)",
        reference_date, num_quarters
    );

    let dates = get_qoq_dates(reference_date, num_quarters)?;
    let available_dates = get_available_dates()?;

    // Filter to only available dates
    let valid_dates: Vec<String> = dates
        .into_iter()
        .filter(|d| available_dates.contains(d))
        .collect();

    if valid_dates.len() < 2 {
        anyhow::bail!(
            "Not enough data for QoQ comparison. Found {} dates, need at least 2.\n\
            Available dates: {:?}\n\
            Use 'fetch-specific-date-market-caps' to fetch the required dates.",
            valid_dates.len(),
            available_dates
        );
    }

    println!("Using {} dates for QoQ analysis:", valid_dates.len());
    for date in &valid_dates {
        println!("  - {}", date);
    }

    let (trends, summary) = analyze_trends(pool, valid_dates.clone()).await?;
    export_trend_analysis(&trends, &summary, &valid_dates)?;

    Ok(())
}

// =====================================================
// Rolling Period Comparison
// =====================================================

/// Perform rolling period comparison
pub async fn compare_rolling(
    _pool: &SqlitePool,
    reference_date: &str,
    period: RollingPeriod,
) -> Result<()> {
    let ref_date = NaiveDate::parse_from_str(reference_date, "%Y-%m-%d")
        .context("Invalid date format. Use YYYY-MM-DD")?;

    let start_date = ref_date - Duration::days(period.days());
    let start_date_str = start_date.format("%Y-%m-%d").to_string();

    println!(
        "Performing {} rolling comparison: {} to {}",
        period.name(),
        start_date_str,
        reference_date
    );

    // Check if we have data for both dates
    let available_dates = get_available_dates()?;

    if !available_dates.contains(&start_date_str) {
        anyhow::bail!(
            "No data found for start date {}. Please run:\n  \
            cargo run -- fetch-specific-date-market-caps {}",
            start_date_str,
            start_date_str
        );
    }

    if !available_dates.contains(&reference_date.to_string()) {
        anyhow::bail!(
            "No data found for reference date {}. Please run:\n  \
            cargo run -- fetch-specific-date-market-caps {}",
            reference_date,
            reference_date
        );
    }

    // Use the existing comparison function
    crate::compare_marketcaps::compare_market_caps(&start_date_str, reference_date).await?;

    Ok(())
}

// =====================================================
// Benchmark Comparison
// =====================================================

/// Benchmark comparison result
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkComparison {
    pub ticker: String,
    pub name: String,
    pub change_pct: Option<f64>,
    pub benchmark_change_pct: f64,
    pub relative_performance: Option<f64>, // Difference from benchmark
    pub beta: Option<f64>,                 // Correlation with benchmark
}

/// Perform benchmark comparison
pub async fn compare_with_benchmark(
    pool: &SqlitePool,
    from_date: &str,
    to_date: &str,
    benchmark: Benchmark,
) -> Result<()> {
    println!(
        "Comparing performance against {} ({}) from {} to {}",
        benchmark.name(),
        benchmark.ticker(),
        from_date,
        to_date
    );

    // Get exchange rates for normalization
    let to_date_parsed = NaiveDate::parse_from_str(to_date, "%Y-%m-%d")?;
    let to_timestamp = NaiveDateTime::new(to_date_parsed, NaiveTime::default())
        .and_utc()
        .timestamp();
    let normalization_rates = get_rate_map_from_db_for_date(pool, Some(to_timestamp)).await?;

    // Load market cap data
    let from_file = find_csv_for_date(from_date)?;
    let to_file = find_csv_for_date(to_date)?;

    let from_records = read_market_cap_csv(&from_file)?;
    let to_records = read_market_cap_csv(&to_file)?;

    let from_map: HashMap<String, MarketCapRecord> = from_records
        .into_iter()
        .map(|r| (r.ticker.clone(), r))
        .collect();
    let to_map: HashMap<String, MarketCapRecord> = to_records
        .into_iter()
        .map(|r| (r.ticker.clone(), r))
        .collect();

    // Calculate benchmark performance
    // Note: Benchmark ticker might not be in our data, so we use total market cap as proxy
    // or we could fetch the actual benchmark data separately
    let total_from: f64 = from_map.values().filter_map(|r| r.market_cap_usd).sum();
    let total_to: f64 = to_map.values().filter_map(|r| r.market_cap_usd).sum();

    let benchmark_change_pct = if total_from > 0.0 {
        ((total_to - total_from) / total_from) * 100.0
    } else {
        0.0
    };

    println!(
        "\n{} proxy performance (total market cap): {:.2}%",
        benchmark.name(),
        benchmark_change_pct
    );

    // Calculate relative performance for each ticker
    let mut comparisons: Vec<BenchmarkComparison> = Vec::new();

    let all_tickers: HashSet<_> = from_map.keys().chain(to_map.keys()).cloned().collect();

    for ticker in all_tickers {
        let from_record = from_map.get(&ticker);
        let to_record = to_map.get(&ticker);

        let name = from_record
            .map(|r| r.name.clone())
            .or_else(|| to_record.map(|r| r.name.clone()))
            .unwrap_or_default();

        let market_cap_from = from_record.and_then(|r| {
            r.market_cap_original.map(|orig| {
                let currency = r.original_currency.as_deref().unwrap_or("USD");
                if normalization_rates.is_empty() {
                    r.market_cap_usd.unwrap_or(orig)
                } else {
                    convert_currency(orig, currency, "USD", &normalization_rates)
                }
            })
        });

        let market_cap_to = to_record.and_then(|r| {
            r.market_cap_original.map(|orig| {
                let currency = r.original_currency.as_deref().unwrap_or("USD");
                if normalization_rates.is_empty() {
                    r.market_cap_usd.unwrap_or(orig)
                } else {
                    convert_currency(orig, currency, "USD", &normalization_rates)
                }
            })
        });

        let change_pct = match (market_cap_from, market_cap_to) {
            (Some(from_val), Some(to_val)) if from_val > 0.0 => {
                Some(((to_val - from_val) / from_val) * 100.0)
            }
            _ => None,
        };

        let relative_performance = change_pct.map(|c| c - benchmark_change_pct);

        comparisons.push(BenchmarkComparison {
            ticker,
            name,
            change_pct,
            benchmark_change_pct,
            relative_performance,
            beta: None, // Would need historical data to calculate
        });
    }

    // Sort by relative performance
    comparisons.sort_by(|a, b| {
        let a_rel = a.relative_performance.unwrap_or(f64::NEG_INFINITY);
        let b_rel = b.relative_performance.unwrap_or(f64::NEG_INFINITY);
        b_rel.partial_cmp(&a_rel).unwrap()
    });

    // Export results
    export_benchmark_comparison(&comparisons, from_date, to_date, &benchmark)?;

    Ok(())
}

/// Export benchmark comparison results
fn export_benchmark_comparison(
    comparisons: &[BenchmarkComparison],
    from_date: &str,
    to_date: &str,
    benchmark: &Benchmark,
) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let benchmark_name = benchmark.name().replace(' ', "_").to_lowercase();
    let csv_filename = format!(
        "output/benchmark_{}_{}_{}_to_{}_{}.csv",
        benchmark_name, from_date, to_date, from_date, timestamp
    );
    let md_filename = format!(
        "output/benchmark_{}_{}_{}_to_{}_summary_{}.md",
        benchmark_name, from_date, to_date, from_date, timestamp
    );

    // Export CSV
    let file = File::create(&csv_filename)?;
    let mut writer = Writer::from_writer(file);

    writer.write_record(&[
        "Ticker",
        "Name",
        "Change (%)",
        "Benchmark Change (%)",
        "Relative Performance (%)",
        "Outperformed",
    ])?;

    for comp in comparisons {
        let outperformed = comp
            .relative_performance
            .map(|r| if r > 0.0 { "Yes" } else { "No" })
            .unwrap_or("N/A");

        writer.write_record(&[
            comp.ticker.clone(),
            comp.name.clone(),
            comp.change_pct
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            format!("{:.2}", comp.benchmark_change_pct),
            comp.relative_performance
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            outperformed.to_string(),
        ])?;
    }
    writer.flush()?;
    println!("Benchmark comparison exported to {}", csv_filename);

    // Export Markdown summary
    let mut file = File::create(&md_filename)?;

    writeln!(
        file,
        "# Benchmark Comparison vs {} ({} to {})",
        benchmark.name(),
        from_date,
        to_date
    )?;
    writeln!(file)?;

    // Summary statistics
    let outperformers = comparisons
        .iter()
        .filter(|c| c.relative_performance.map(|r| r > 0.0).unwrap_or(false))
        .count();
    let underperformers = comparisons
        .iter()
        .filter(|c| c.relative_performance.map(|r| r < 0.0).unwrap_or(false))
        .count();

    writeln!(file, "## Summary")?;
    writeln!(
        file,
        "- **Benchmark**: {} (proxy: total market cap)",
        benchmark.name()
    )?;
    writeln!(
        file,
        "- **Benchmark Return**: {:.2}%",
        comparisons
            .first()
            .map(|c| c.benchmark_change_pct)
            .unwrap_or(0.0)
    )?;
    writeln!(file, "- **Outperformers**: {}", outperformers)?;
    writeln!(file, "- **Underperformers**: {}", underperformers)?;
    writeln!(file)?;

    writeln!(file, "## Top 10 Outperformers")?;
    writeln!(file, "| Ticker | Name | Return (%) | Relative (%) |")?;
    writeln!(file, "|--------|------|------------|--------------|")?;
    for comp in comparisons
        .iter()
        .filter(|c| c.relative_performance.map(|r| r > 0.0).unwrap_or(false))
        .take(10)
    {
        writeln!(
            file,
            "| [{}](https://finance.yahoo.com/quote/{}/) | {} | {:.2}% | +{:.2}% |",
            comp.ticker,
            comp.ticker,
            comp.name,
            comp.change_pct.unwrap_or(0.0),
            comp.relative_performance.unwrap_or(0.0)
        )?;
    }
    writeln!(file)?;

    writeln!(file, "## Top 10 Underperformers")?;
    writeln!(file, "| Ticker | Name | Return (%) | Relative (%) |")?;
    writeln!(file, "|--------|------|------------|--------------|")?;
    for comp in comparisons
        .iter()
        .filter(|c| c.relative_performance.map(|r| r < 0.0).unwrap_or(false))
        .rev()
        .take(10)
    {
        writeln!(
            file,
            "| [{}](https://finance.yahoo.com/quote/{}/) | {} | {:.2}% | {:.2}% |",
            comp.ticker,
            comp.ticker,
            comp.name,
            comp.change_pct.unwrap_or(0.0),
            comp.relative_performance.unwrap_or(0.0)
        )?;
    }
    writeln!(file)?;

    writeln!(file, "---")?;
    writeln!(
        file,
        "*Generated on {}*",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    println!("Summary report exported to {}", md_filename);

    Ok(())
}

// =====================================================
// Peer Group Comparison
// =====================================================

/// Peer group comparison result
#[derive(Debug, Clone, Serialize)]
pub struct PeerGroupResult {
    pub group_name: String,
    pub total_market_cap_from: f64,
    pub total_market_cap_to: f64,
    pub total_change_pct: f64,
    pub avg_change_pct: f64,
    pub best_performer: Option<(String, f64)>,
    pub worst_performer: Option<(String, f64)>,
    pub members: Vec<PeerMemberResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerMemberResult {
    pub ticker: String,
    pub name: String,
    pub market_cap_from: Option<f64>,
    pub market_cap_to: Option<f64>,
    pub change_pct: Option<f64>,
    pub rank_from: Option<usize>,
    pub rank_to: Option<usize>,
}

/// Perform peer group comparison
pub async fn compare_peer_groups(
    pool: &SqlitePool,
    from_date: &str,
    to_date: &str,
    groups: Option<Vec<String>>, // None = all predefined groups
) -> Result<()> {
    println!(
        "Performing peer group comparison from {} to {}",
        from_date, to_date
    );

    let peer_groups = get_predefined_peer_groups();

    // Filter groups if specified
    let selected_groups: Vec<PeerGroup> = if let Some(group_names) = groups {
        peer_groups
            .into_iter()
            .filter(|g| group_names.iter().any(|n| n.eq_ignore_ascii_case(&g.name)))
            .collect()
    } else {
        peer_groups
    };

    if selected_groups.is_empty() {
        anyhow::bail!("No peer groups found. Available groups: Luxury, Sportswear, Fast Fashion, Department Stores, Value Retail, Footwear, E-commerce, Asian Fashion");
    }

    // Get exchange rates
    let to_date_parsed = NaiveDate::parse_from_str(to_date, "%Y-%m-%d")?;
    let to_timestamp = NaiveDateTime::new(to_date_parsed, NaiveTime::default())
        .and_utc()
        .timestamp();
    let normalization_rates = get_rate_map_from_db_for_date(pool, Some(to_timestamp)).await?;

    // Load market cap data
    let from_file = find_csv_for_date(from_date)?;
    let to_file = find_csv_for_date(to_date)?;

    let from_records = read_market_cap_csv(&from_file)?;
    let to_records = read_market_cap_csv(&to_file)?;

    let from_map: HashMap<String, MarketCapRecord> = from_records
        .into_iter()
        .map(|r| (r.ticker.clone(), r))
        .collect();
    let to_map: HashMap<String, MarketCapRecord> = to_records
        .into_iter()
        .map(|r| (r.ticker.clone(), r))
        .collect();

    // Analyze each peer group
    let mut results: Vec<PeerGroupResult> = Vec::new();

    for group in &selected_groups {
        println!("  Analyzing {} group...", group.name);

        let mut members: Vec<PeerMemberResult> = Vec::new();
        let mut total_from = 0.0f64;
        let mut total_to = 0.0f64;
        let mut changes: Vec<f64> = Vec::new();

        for ticker in &group.tickers {
            let from_record = from_map.get(ticker);
            let to_record = to_map.get(ticker);

            let name = from_record
                .map(|r| r.name.clone())
                .or_else(|| to_record.map(|r| r.name.clone()))
                .unwrap_or_else(|| ticker.clone());

            let market_cap_from = from_record.and_then(|r| {
                r.market_cap_original.map(|orig| {
                    let currency = r.original_currency.as_deref().unwrap_or("USD");
                    if normalization_rates.is_empty() {
                        r.market_cap_usd.unwrap_or(orig)
                    } else {
                        convert_currency(orig, currency, "USD", &normalization_rates)
                    }
                })
            });

            let market_cap_to = to_record.and_then(|r| {
                r.market_cap_original.map(|orig| {
                    let currency = r.original_currency.as_deref().unwrap_or("USD");
                    if normalization_rates.is_empty() {
                        r.market_cap_usd.unwrap_or(orig)
                    } else {
                        convert_currency(orig, currency, "USD", &normalization_rates)
                    }
                })
            });

            let change_pct = match (market_cap_from, market_cap_to) {
                (Some(from_val), Some(to_val)) if from_val > 0.0 => {
                    let pct = ((to_val - from_val) / from_val) * 100.0;
                    changes.push(pct);
                    Some(pct)
                }
                _ => None,
            };

            if let Some(mf) = market_cap_from {
                total_from += mf;
            }
            if let Some(mt) = market_cap_to {
                total_to += mt;
            }

            members.push(PeerMemberResult {
                ticker: ticker.clone(),
                name,
                market_cap_from,
                market_cap_to,
                change_pct,
                rank_from: from_record.and_then(|r| r.rank),
                rank_to: to_record.and_then(|r| r.rank),
            });
        }

        // Sort members by change percentage
        members.sort_by(|a, b| {
            let a_pct = a.change_pct.unwrap_or(f64::NEG_INFINITY);
            let b_pct = b.change_pct.unwrap_or(f64::NEG_INFINITY);
            b_pct.partial_cmp(&a_pct).unwrap()
        });

        let total_change_pct = if total_from > 0.0 {
            ((total_to - total_from) / total_from) * 100.0
        } else {
            0.0
        };

        let avg_change_pct = if !changes.is_empty() {
            changes.iter().sum::<f64>() / changes.len() as f64
        } else {
            0.0
        };

        let best = members
            .first()
            .and_then(|m| m.change_pct.map(|p| (m.ticker.clone(), p)));
        let worst = members
            .last()
            .and_then(|m| m.change_pct.map(|p| (m.ticker.clone(), p)));

        results.push(PeerGroupResult {
            group_name: group.name.clone(),
            total_market_cap_from: total_from,
            total_market_cap_to: total_to,
            total_change_pct,
            avg_change_pct,
            best_performer: best,
            worst_performer: worst,
            members,
        });
    }

    // Sort groups by performance
    results.sort_by(|a, b| b.total_change_pct.partial_cmp(&a.total_change_pct).unwrap());

    // Export results
    export_peer_group_comparison(&results, from_date, to_date)?;

    Ok(())
}

/// Export peer group comparison results
fn export_peer_group_comparison(
    results: &[PeerGroupResult],
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_filename = format!(
        "output/peer_groups_{}_to_{}_{}.csv",
        from_date, to_date, timestamp
    );
    let md_filename = format!(
        "output/peer_groups_{}_to_{}_summary_{}.md",
        from_date, to_date, timestamp
    );

    // Export CSV
    let file = File::create(&csv_filename)?;
    let mut writer = Writer::from_writer(file);

    writer.write_record(&[
        "Group",
        "Ticker",
        "Name",
        "Market Cap From ($)",
        "Market Cap To ($)",
        "Change (%)",
        "Rank From",
        "Rank To",
    ])?;

    for result in results {
        for member in &result.members {
            writer.write_record(&[
                result.group_name.clone(),
                member.ticker.clone(),
                member.name.clone(),
                member
                    .market_cap_from
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "N/A".to_string()),
                member
                    .market_cap_to
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "N/A".to_string()),
                member
                    .change_pct
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "N/A".to_string()),
                member
                    .rank_from
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                member
                    .rank_to
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            ])?;
        }
    }
    writer.flush()?;
    println!("Peer group data exported to {}", csv_filename);

    // Export Markdown summary
    let mut file = File::create(&md_filename)?;

    writeln!(
        file,
        "# Peer Group Comparison: {} to {}",
        from_date, to_date
    )?;
    writeln!(file)?;

    writeln!(file, "## Group Performance Summary")?;
    writeln!(
        file,
        "| Group | Market Cap Change | Avg Stock Change | Best | Worst |"
    )?;
    writeln!(
        file,
        "|-------|-------------------|------------------|------|-------|"
    )?;

    for result in results {
        let best = result
            .best_performer
            .as_ref()
            .map(|(t, p)| format!("{} (+{:.1}%)", t, p))
            .unwrap_or_else(|| "N/A".to_string());
        let worst = result
            .worst_performer
            .as_ref()
            .map(|(t, p)| format!("{} ({:.1}%)", t, p))
            .unwrap_or_else(|| "N/A".to_string());

        writeln!(
            file,
            "| {} | {:.2}% | {:.2}% | {} | {} |",
            result.group_name, result.total_change_pct, result.avg_change_pct, best, worst
        )?;
    }
    writeln!(file)?;

    // Detailed breakdown for each group
    for result in results {
        writeln!(file, "## {}", result.group_name)?;
        writeln!(file)?;
        writeln!(
            file,
            "- **Total Market Cap (Start)**: ${:.2}B",
            result.total_market_cap_from / 1_000_000_000.0
        )?;
        writeln!(
            file,
            "- **Total Market Cap (End)**: ${:.2}B",
            result.total_market_cap_to / 1_000_000_000.0
        )?;
        writeln!(file, "- **Group Change**: {:.2}%", result.total_change_pct)?;
        writeln!(file)?;

        writeln!(file, "| Ticker | Name | Change (%) | Market Cap To |")?;
        writeln!(file, "|--------|------|------------|---------------|")?;

        for member in &result.members {
            writeln!(
                file,
                "| [{}](https://finance.yahoo.com/quote/{}/) | {} | {}% | {} |",
                member.ticker,
                member.ticker,
                member.name,
                member
                    .change_pct
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "N/A".to_string()),
                member
                    .market_cap_to
                    .map(|v| format!("${:.2}B", v / 1_000_000_000.0))
                    .unwrap_or_else(|| "N/A".to_string())
            )?;
        }
        writeln!(file)?;
    }

    writeln!(file, "---")?;
    writeln!(
        file,
        "*Generated on {}*",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    println!("Summary report exported to {}", md_filename);

    Ok(())
}

// =====================================================
// Multi-date comparison command (wrapper)
// =====================================================

/// Multi-date trend analysis command
pub async fn multi_date_comparison(pool: &SqlitePool, dates: Vec<String>) -> Result<()> {
    let (trends, summary) = analyze_trends(pool, dates.clone()).await?;
    export_trend_analysis(&trends, &summary, &dates)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_yoy_dates() {
        let dates = get_yoy_dates("2025-06-15", 3).unwrap();
        assert_eq!(dates.len(), 4);
        assert_eq!(dates[0], "2022-06-15");
        assert_eq!(dates[3], "2025-06-15");
    }

    #[test]
    fn test_get_yoy_dates_leap_year() {
        let dates = get_yoy_dates("2024-02-29", 2).unwrap();
        assert_eq!(dates.len(), 3);
        // Feb 29 should become Feb 28 on non-leap years
        assert_eq!(dates[0], "2022-02-28");
        assert_eq!(dates[1], "2023-02-28");
        assert_eq!(dates[2], "2024-02-28");
    }

    #[test]
    fn test_get_qoq_dates() {
        let dates = get_qoq_dates("2025-06-15", 4).unwrap();
        assert!(dates.len() >= 2);
        // Should contain quarter-end dates
        for date in &dates {
            let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
            assert!(
                (d.month() == 3 && d.day() == 31)
                    || (d.month() == 6 && d.day() == 30)
                    || (d.month() == 9 && d.day() == 30)
                    || (d.month() == 12 && d.day() == 31)
            );
        }
    }

    #[test]
    fn test_rolling_period_days() {
        assert_eq!(RollingPeriod::Days30.days(), 30);
        assert_eq!(RollingPeriod::Days90.days(), 90);
        assert_eq!(RollingPeriod::Year1.days(), 365);
        assert_eq!(RollingPeriod::Custom(45).days(), 45);
    }

    #[test]
    fn test_get_predefined_peer_groups() {
        let groups = get_predefined_peer_groups();
        assert!(!groups.is_empty());

        let luxury = groups.iter().find(|g| g.name == "Luxury");
        assert!(luxury.is_some());
        assert!(luxury.unwrap().tickers.contains(&"MC.PA".to_string()));

        let sportswear = groups.iter().find(|g| g.name == "Sportswear");
        assert!(sportswear.is_some());
        assert!(sportswear.unwrap().tickers.contains(&"NKE".to_string()));
    }

    #[test]
    fn test_benchmark_names() {
        assert_eq!(Benchmark::SP500.name(), "S&P 500");
        assert_eq!(Benchmark::MSCI.name(), "MSCI World");
        assert_eq!(Benchmark::Custom("Test".to_string()).name(), "Test");
    }

    #[test]
    fn test_quarter_end_calculation() {
        // Q1
        let q1 = get_quarter_end(NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()).unwrap();
        assert_eq!(q1, NaiveDate::from_ymd_opt(2025, 3, 31).unwrap());

        // Q2
        let q2 = get_quarter_end(NaiveDate::from_ymd_opt(2025, 5, 20).unwrap()).unwrap();
        assert_eq!(q2, NaiveDate::from_ymd_opt(2025, 6, 30).unwrap());

        // Q3
        let q3 = get_quarter_end(NaiveDate::from_ymd_opt(2025, 8, 1).unwrap()).unwrap();
        assert_eq!(q3, NaiveDate::from_ymd_opt(2025, 9, 30).unwrap());

        // Q4
        let q4 = get_quarter_end(NaiveDate::from_ymd_opt(2025, 11, 30).unwrap()).unwrap();
        assert_eq!(q4, NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
    }
}
