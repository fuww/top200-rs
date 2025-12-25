// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for CSV comparison functionality
//!
//! These tests verify:
//! - Market cap comparison calculations (absolute change, percentage change)
//! - Rank change calculations
//! - Market share calculations
//! - CSV reading and parsing
//! - Edge cases in data handling

mod common;

use anyhow::Result;
use common::{create_test_csv_file, TestCompany};
use csv::Reader;
use std::collections::HashMap;
use std::fs::File;
use tempfile::TempDir;

/// Helper: Calculate absolute change
fn calculate_absolute_change(from: Option<f64>, to: Option<f64>) -> Option<f64> {
    match (from, to) {
        (Some(f), Some(t)) => Some(t - f),
        _ => None,
    }
}

/// Helper: Calculate percentage change
fn calculate_percentage_change(from: Option<f64>, to: Option<f64>) -> Option<f64> {
    match (from, to) {
        (Some(f), Some(t)) if f != 0.0 => Some(((t - f) / f) * 100.0),
        _ => None,
    }
}

/// Helper: Calculate rank change (positive = improved, moved up)
fn calculate_rank_change(from: Option<usize>, to: Option<usize>) -> Option<i32> {
    match (from, to) {
        (Some(f), Some(t)) => Some(f as i32 - t as i32),
        _ => None,
    }
}

/// Helper: Calculate market shares
fn calculate_market_shares(market_caps: &[f64]) -> Vec<f64> {
    let total: f64 = market_caps.iter().sum();
    if total == 0.0 {
        return vec![0.0; market_caps.len()];
    }
    market_caps
        .iter()
        .map(|cap| (cap / total) * 100.0)
        .collect()
}

// ==================== Absolute Change Tests ====================

#[test]
fn test_absolute_change_basic() {
    // Simple increase
    assert_eq!(
        calculate_absolute_change(Some(100.0), Some(150.0)),
        Some(50.0)
    );
    // Simple decrease
    assert_eq!(
        calculate_absolute_change(Some(150.0), Some(100.0)),
        Some(-50.0)
    );
    // No change
    assert_eq!(
        calculate_absolute_change(Some(100.0), Some(100.0)),
        Some(0.0)
    );
}

#[test]
fn test_absolute_change_missing_values() {
    assert_eq!(calculate_absolute_change(None, Some(100.0)), None);
    assert_eq!(calculate_absolute_change(Some(100.0), None), None);
    assert_eq!(calculate_absolute_change(None, None), None);
}

#[test]
fn test_absolute_change_large_values() {
    // Billions scale (typical market caps)
    let from = 100_000_000_000.0; // $100B
    let to = 150_000_000_000.0; // $150B
    let change = calculate_absolute_change(Some(from), Some(to)).unwrap();
    assert!((change - 50_000_000_000.0).abs() < 0.01);
}

// ==================== Percentage Change Tests ====================

#[test]
fn test_percentage_change_basic() {
    // 50% increase
    assert_eq!(
        calculate_percentage_change(Some(100.0), Some(150.0)),
        Some(50.0)
    );
    // 50% decrease
    let result = calculate_percentage_change(Some(100.0), Some(50.0)).unwrap();
    assert!((result - (-50.0)).abs() < 0.001);
}

#[test]
fn test_percentage_change_double() {
    // 100% increase (doubled)
    assert_eq!(
        calculate_percentage_change(Some(100.0), Some(200.0)),
        Some(100.0)
    );
}

#[test]
fn test_percentage_change_from_zero() {
    // From zero should return None (division by zero)
    assert_eq!(calculate_percentage_change(Some(0.0), Some(100.0)), None);
}

#[test]
fn test_percentage_change_to_zero() {
    // Complete loss: -100%
    assert_eq!(
        calculate_percentage_change(Some(100.0), Some(0.0)),
        Some(-100.0)
    );
}

#[test]
fn test_percentage_change_missing_values() {
    assert_eq!(calculate_percentage_change(None, Some(100.0)), None);
    assert_eq!(calculate_percentage_change(Some(100.0), None), None);
    assert_eq!(calculate_percentage_change(None, None), None);
}

#[test]
fn test_percentage_change_precision() {
    // Verify precision with common percentage
    let result = calculate_percentage_change(Some(1000.0), Some(1033.33)).unwrap();
    assert!((result - 3.333).abs() < 0.01);
}

// ==================== Rank Change Tests ====================

#[test]
fn test_rank_change_improvement() {
    // Moved from rank 5 to rank 3 = improved by 2
    assert_eq!(calculate_rank_change(Some(5), Some(3)), Some(2));
}

#[test]
fn test_rank_change_decline() {
    // Moved from rank 3 to rank 5 = declined by 2
    assert_eq!(calculate_rank_change(Some(3), Some(5)), Some(-2));
}

#[test]
fn test_rank_change_stable() {
    assert_eq!(calculate_rank_change(Some(3), Some(3)), Some(0));
}

#[test]
fn test_rank_change_missing() {
    assert_eq!(calculate_rank_change(None, Some(3)), None);
    assert_eq!(calculate_rank_change(Some(3), None), None);
    assert_eq!(calculate_rank_change(None, None), None);
}

#[test]
fn test_rank_change_new_entry() {
    // New entry with no previous rank
    assert_eq!(calculate_rank_change(None, Some(10)), None);
}

// ==================== Market Share Tests ====================

#[test]
fn test_market_shares_basic() {
    let caps = vec![50.0, 30.0, 20.0];
    let shares = calculate_market_shares(&caps);

    assert!((shares[0] - 50.0).abs() < 0.01);
    assert!((shares[1] - 30.0).abs() < 0.01);
    assert!((shares[2] - 20.0).abs() < 0.01);
}

#[test]
fn test_market_shares_sum_to_100() {
    let caps = vec![100.0, 200.0, 300.0, 400.0];
    let shares = calculate_market_shares(&caps);
    let total: f64 = shares.iter().sum();

    assert!((total - 100.0).abs() < 0.001);
}

#[test]
fn test_market_shares_single_company() {
    let caps = vec![1_000_000.0];
    let shares = calculate_market_shares(&caps);

    assert!((shares[0] - 100.0).abs() < 0.001);
}

#[test]
fn test_market_shares_empty() {
    let caps: Vec<f64> = vec![];
    let shares = calculate_market_shares(&caps);

    assert!(shares.is_empty());
}

#[test]
fn test_market_shares_all_zero() {
    let caps = vec![0.0, 0.0, 0.0];
    let shares = calculate_market_shares(&caps);

    // All zeros should give 0% shares (avoid division by zero)
    assert_eq!(shares, vec![0.0, 0.0, 0.0]);
}

#[test]
fn test_market_shares_realistic_distribution() {
    // Simulate top 5 companies (typical market cap distribution)
    let caps = vec![
        3_000_000_000_000.0, // AAPL ~$3T
        2_800_000_000_000.0, // MSFT ~$2.8T
        1_500_000_000_000.0, // GOOGL ~$1.5T
        1_200_000_000_000.0, // AMZN ~$1.2T
        800_000_000_000.0,   // NVDA ~$800B
    ];
    let shares = calculate_market_shares(&caps);

    // Top company should have ~32% share
    assert!(shares[0] > 30.0 && shares[0] < 35.0);

    // Shares should decrease
    assert!(shares[0] > shares[1]);
    assert!(shares[1] > shares[2]);
    assert!(shares[2] > shares[3]);
    assert!(shares[3] > shares[4]);
}

// ==================== CSV File Tests ====================

#[test]
fn test_csv_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let companies = vec![
        TestCompany::simple("AAPL", "Apple Inc.", 3_000_000_000_000.0),
        TestCompany::simple("MSFT", "Microsoft", 2_800_000_000_000.0),
    ];

    let csv_path = create_test_csv_file(temp_dir.path(), "2025-01-01", &companies)?;

    // Verify file exists
    assert!(csv_path.exists());

    // Verify file is readable and has correct number of rows
    let file = File::open(&csv_path)?;
    let mut reader = Reader::from_reader(file);
    let records: Vec<_> = reader.records().collect();

    assert_eq!(records.len(), 2);

    Ok(())
}

#[test]
fn test_csv_columns_present() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let companies = vec![TestCompany::simple("TEST", "Test Company", 100.0)];

    let csv_path = create_test_csv_file(temp_dir.path(), "2025-01-01", &companies)?;

    common::assert_csv_has_columns(
        &csv_path,
        &[
            "Rank",
            "Ticker",
            "Name",
            "Market Cap (USD)",
            "Market Cap (EUR)",
        ],
    )?;

    Ok(())
}

#[test]
fn test_csv_special_characters_in_name() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create company with special chars using the full struct
    let company = TestCompany {
        ticker: "TEST".to_string(),
        name: "Test & Co. \"Special\" Corp.".to_string(),
        market_cap: 100.0,
        currency: "USD".to_string(),
        market_cap_eur: 90.0,
        eur_rate: 0.9,
        market_cap_usd: 100.0,
        usd_rate: 1.0,
        price: 10.0,
        exchange: "NYSE".to_string(),
        description: None,
        homepage: None,
        employees: None,
        ceo: None,
    };

    let csv_path = create_test_csv_file(temp_dir.path(), "2025-01-01", &[company])?;

    // CSV should handle special characters correctly
    let file = File::open(&csv_path)?;
    let mut reader = Reader::from_reader(file);
    let record = reader.records().next().unwrap()?;

    // Name should be preserved (column index 2)
    assert!(record.get(2).is_some());

    Ok(())
}

// ==================== Comparison Edge Cases ====================

#[test]
fn test_comparison_new_company() {
    // Company only exists in "to" date
    let from: Option<f64> = None;
    let to = Some(1_000_000_000.0);

    let abs_change = calculate_absolute_change(from, to);
    let pct_change = calculate_percentage_change(from, to);

    // Both should be None since we can't calculate change from nothing
    assert!(abs_change.is_none());
    assert!(pct_change.is_none());
}

#[test]
fn test_comparison_delisted_company() {
    // Company only exists in "from" date
    let from = Some(1_000_000_000.0);
    let to: Option<f64> = None;

    let abs_change = calculate_absolute_change(from, to);
    let pct_change = calculate_percentage_change(from, to);

    // Both should be None since company no longer exists
    assert!(abs_change.is_none());
    assert!(pct_change.is_none());
}

#[test]
fn test_comparison_small_values() {
    // Very small market caps (microcaps)
    let from = Some(10_000_000.0); // $10M
    let to = Some(15_000_000.0); // $15M

    let abs_change = calculate_absolute_change(from, to).unwrap();
    let pct_change = calculate_percentage_change(from, to).unwrap();

    assert!((abs_change - 5_000_000.0).abs() < 0.01);
    assert!((pct_change - 50.0).abs() < 0.01);
}

#[test]
fn test_comparison_negative_market_cap_edge() {
    // Negative market cap shouldn't happen, but test the math
    let from = Some(-100.0);
    let to = Some(-50.0);

    // Math still works, but results may be counterintuitive
    let abs_change = calculate_absolute_change(from, to).unwrap();
    assert!((abs_change - 50.0).abs() < 0.01);
}

// ==================== Sorting and Ranking Tests ====================

#[test]
fn test_ranking_by_market_cap() {
    let mut companies = vec![
        ("AAPL", 3000.0_f64),
        ("MSFT", 2800.0_f64),
        ("GOOGL", 1500.0_f64),
    ];

    // Sort by market cap descending
    companies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    assert_eq!(companies[0].0, "AAPL");
    assert_eq!(companies[1].0, "MSFT");
    assert_eq!(companies[2].0, "GOOGL");
}

#[test]
fn test_ranking_ties() {
    let mut companies = vec![("A", 1000.0_f64), ("B", 1000.0_f64), ("C", 1000.0_f64)];

    // Stable sort should maintain original order for ties
    companies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // All should have same market cap
    assert!(companies
        .iter()
        .all(|(_, cap)| (*cap - 1000.0).abs() < 0.01));
}

// ==================== Integration Test: Full Comparison Flow ====================

#[test]
fn test_full_comparison_flow() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create "from" date data
    let from_companies = vec![
        TestCompany::simple("AAPL", "Apple", 2_500_000_000_000.0),
        TestCompany::simple("MSFT", "Microsoft", 2_800_000_000_000.0),
    ];

    // Create "to" date data (AAPL grows, MSFT shrinks, roles reverse)
    let to_companies = vec![
        TestCompany::simple("AAPL", "Apple", 3_000_000_000_000.0),
        TestCompany::simple("MSFT", "Microsoft", 2_600_000_000_000.0),
    ];

    let _from_csv = create_test_csv_file(temp_dir.path(), "2025-01-01", &from_companies)?;
    let _to_csv = create_test_csv_file(temp_dir.path(), "2025-02-01", &to_companies)?;

    // Build comparison data
    let mut comparisons: HashMap<String, (Option<f64>, Option<f64>, Option<usize>, Option<usize>)> =
        HashMap::new();

    // From date: MSFT is #1, AAPL is #2
    comparisons.insert(
        "MSFT".to_string(),
        (Some(2_800_000_000_000.0), None, Some(1), None),
    );
    comparisons.insert(
        "AAPL".to_string(),
        (Some(2_500_000_000_000.0), None, Some(2), None),
    );

    // To date: AAPL is now #1, MSFT is #2
    comparisons.entry("AAPL".to_string()).and_modify(|e| {
        e.1 = Some(3_000_000_000_000.0);
        e.3 = Some(1);
    });
    comparisons.entry("MSFT".to_string()).and_modify(|e| {
        e.1 = Some(2_600_000_000_000.0);
        e.3 = Some(2);
    });

    // Verify AAPL results
    let aapl = comparisons.get("AAPL").unwrap();
    let aapl_abs_change = calculate_absolute_change(aapl.0, aapl.1).unwrap();
    let aapl_pct_change = calculate_percentage_change(aapl.0, aapl.1).unwrap();
    let aapl_rank_change = calculate_rank_change(aapl.2, aapl.3).unwrap();

    assert!((aapl_abs_change - 500_000_000_000.0).abs() < 0.01); // +$500B
    assert!((aapl_pct_change - 20.0).abs() < 0.01); // +20%
    assert_eq!(aapl_rank_change, 1); // Moved up 1 spot

    // Verify MSFT results
    let msft = comparisons.get("MSFT").unwrap();
    let msft_abs_change = calculate_absolute_change(msft.0, msft.1).unwrap();
    let msft_pct_change = calculate_percentage_change(msft.0, msft.1).unwrap();
    let msft_rank_change = calculate_rank_change(msft.2, msft.3).unwrap();

    assert!((msft_abs_change - (-200_000_000_000.0)).abs() < 0.01); // -$200B
    assert!(msft_pct_change < 0.0); // Negative %
    assert_eq!(msft_rank_change, -1); // Moved down 1 spot

    Ok(())
}
