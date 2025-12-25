// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for visualization data preparation and chart logic
//!
//! These tests focus on:
//! - Data preparation for charts
//! - Sorting and filtering for top gainers/losers
//! - Color selection logic
//! - Data validation before rendering

use std::collections::HashMap;

// ==================== Chart Data Structures ====================

#[derive(Debug, Clone)]
struct ChartDataPoint {
    ticker: String,
    name: String,
    value: f64,
}

#[derive(Debug, Clone)]
struct GainerLoser {
    ticker: String,
    name: String,
    percentage_change: f64,
    absolute_change: f64,
}

#[derive(Debug, Clone)]
struct RankMovement {
    ticker: String,
    name: String,
    rank_from: usize,
    rank_to: usize,
    rank_change: i32,
}

// ==================== Helper Functions ====================

/// Get top N gainers from comparison data
fn get_top_gainers(data: &[GainerLoser], n: usize) -> Vec<GainerLoser> {
    let mut sorted: Vec<_> = data
        .iter()
        .filter(|d| d.percentage_change > 0.0)
        .cloned()
        .collect();
    sorted.sort_by(|a, b| {
        b.percentage_change
            .partial_cmp(&a.percentage_change)
            .unwrap()
    });
    sorted.truncate(n);
    sorted
}

/// Get top N losers from comparison data
fn get_top_losers(data: &[GainerLoser], n: usize) -> Vec<GainerLoser> {
    let mut sorted: Vec<_> = data
        .iter()
        .filter(|d| d.percentage_change < 0.0)
        .cloned()
        .collect();
    sorted.sort_by(|a, b| {
        a.percentage_change
            .partial_cmp(&b.percentage_change)
            .unwrap()
    });
    sorted.truncate(n);
    sorted
}

/// Get top N rank improvements
fn get_top_rank_improvements(data: &[RankMovement], n: usize) -> Vec<RankMovement> {
    let mut sorted: Vec<_> = data.iter().filter(|d| d.rank_change > 0).cloned().collect();
    sorted.sort_by(|a, b| b.rank_change.cmp(&a.rank_change));
    sorted.truncate(n);
    sorted
}

/// Get top N rank declines
fn get_top_rank_declines(data: &[RankMovement], n: usize) -> Vec<RankMovement> {
    let mut sorted: Vec<_> = data.iter().filter(|d| d.rank_change < 0).cloned().collect();
    sorted.sort_by(|a, b| a.rank_change.cmp(&b.rank_change));
    sorted.truncate(n);
    sorted
}

/// Calculate "others" category for donut chart
fn calculate_others(data: &[ChartDataPoint], top_n: usize) -> f64 {
    if data.len() <= top_n {
        return 0.0;
    }
    data[top_n..].iter().map(|d| d.value).sum()
}

/// Format large number for display (e.g., $1.5T, $500B, $100M)
fn format_market_cap(value: f64) -> String {
    if value >= 1_000_000_000_000.0 {
        format!("${:.1}T", value / 1_000_000_000_000.0)
    } else if value >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else {
        format!("${:.0}", value)
    }
}

/// Select color index for chart segment
fn get_color_index(index: usize, total_colors: usize) -> usize {
    index % total_colors
}

// ==================== Gainer/Loser Tests ====================

#[test]
fn test_get_top_gainers_basic() {
    let data = vec![
        GainerLoser {
            ticker: "A".to_string(),
            name: "A Inc".to_string(),
            percentage_change: 10.0,
            absolute_change: 100.0,
        },
        GainerLoser {
            ticker: "B".to_string(),
            name: "B Inc".to_string(),
            percentage_change: 20.0,
            absolute_change: 200.0,
        },
        GainerLoser {
            ticker: "C".to_string(),
            name: "C Inc".to_string(),
            percentage_change: 5.0,
            absolute_change: 50.0,
        },
    ];

    let top = get_top_gainers(&data, 2);

    assert_eq!(top.len(), 2);
    assert_eq!(top[0].ticker, "B"); // 20% is highest
    assert_eq!(top[1].ticker, "A"); // 10% is second
}

#[test]
fn test_get_top_gainers_excludes_losers() {
    let data = vec![
        GainerLoser {
            ticker: "A".to_string(),
            name: "A Inc".to_string(),
            percentage_change: 10.0,
            absolute_change: 100.0,
        },
        GainerLoser {
            ticker: "B".to_string(),
            name: "B Inc".to_string(),
            percentage_change: -5.0,
            absolute_change: -50.0,
        },
        GainerLoser {
            ticker: "C".to_string(),
            name: "C Inc".to_string(),
            percentage_change: 5.0,
            absolute_change: 50.0,
        },
    ];

    let top = get_top_gainers(&data, 10);

    assert_eq!(top.len(), 2); // Only A and C are gainers
    assert!(top.iter().all(|g| g.percentage_change > 0.0));
}

#[test]
fn test_get_top_gainers_request_more_than_available() {
    let data = vec![GainerLoser {
        ticker: "A".to_string(),
        name: "A Inc".to_string(),
        percentage_change: 10.0,
        absolute_change: 100.0,
    }];

    let top = get_top_gainers(&data, 10);

    assert_eq!(top.len(), 1); // Only 1 available
}

#[test]
fn test_get_top_losers_basic() {
    let data = vec![
        GainerLoser {
            ticker: "A".to_string(),
            name: "A Inc".to_string(),
            percentage_change: -10.0,
            absolute_change: -100.0,
        },
        GainerLoser {
            ticker: "B".to_string(),
            name: "B Inc".to_string(),
            percentage_change: -20.0,
            absolute_change: -200.0,
        },
        GainerLoser {
            ticker: "C".to_string(),
            name: "C Inc".to_string(),
            percentage_change: -5.0,
            absolute_change: -50.0,
        },
    ];

    let top = get_top_losers(&data, 2);

    assert_eq!(top.len(), 2);
    assert_eq!(top[0].ticker, "B"); // -20% is worst
    assert_eq!(top[1].ticker, "A"); // -10% is second worst
}

#[test]
fn test_get_top_losers_excludes_gainers() {
    let data = vec![
        GainerLoser {
            ticker: "A".to_string(),
            name: "A Inc".to_string(),
            percentage_change: -10.0,
            absolute_change: -100.0,
        },
        GainerLoser {
            ticker: "B".to_string(),
            name: "B Inc".to_string(),
            percentage_change: 5.0,
            absolute_change: 50.0,
        },
        GainerLoser {
            ticker: "C".to_string(),
            name: "C Inc".to_string(),
            percentage_change: -5.0,
            absolute_change: -50.0,
        },
    ];

    let top = get_top_losers(&data, 10);

    assert_eq!(top.len(), 2); // Only A and C are losers
    assert!(top.iter().all(|l| l.percentage_change < 0.0));
}

// ==================== Rank Movement Tests ====================

#[test]
fn test_get_top_rank_improvements() {
    let data = vec![
        RankMovement {
            ticker: "A".to_string(),
            name: "A".to_string(),
            rank_from: 10,
            rank_to: 5,
            rank_change: 5,
        },
        RankMovement {
            ticker: "B".to_string(),
            name: "B".to_string(),
            rank_from: 15,
            rank_to: 3,
            rank_change: 12,
        },
        RankMovement {
            ticker: "C".to_string(),
            name: "C".to_string(),
            rank_from: 8,
            rank_to: 6,
            rank_change: 2,
        },
    ];

    let top = get_top_rank_improvements(&data, 2);

    assert_eq!(top.len(), 2);
    assert_eq!(top[0].ticker, "B"); // +12 ranks
    assert_eq!(top[1].ticker, "A"); // +5 ranks
}

#[test]
fn test_get_top_rank_declines() {
    let data = vec![
        RankMovement {
            ticker: "A".to_string(),
            name: "A".to_string(),
            rank_from: 5,
            rank_to: 10,
            rank_change: -5,
        },
        RankMovement {
            ticker: "B".to_string(),
            name: "B".to_string(),
            rank_from: 3,
            rank_to: 15,
            rank_change: -12,
        },
        RankMovement {
            ticker: "C".to_string(),
            name: "C".to_string(),
            rank_from: 6,
            rank_to: 8,
            rank_change: -2,
        },
    ];

    let top = get_top_rank_declines(&data, 2);

    assert_eq!(top.len(), 2);
    assert_eq!(top[0].ticker, "B"); // -12 ranks (worst)
    assert_eq!(top[1].ticker, "A"); // -5 ranks
}

#[test]
fn test_rank_movement_excludes_unchanged() {
    let data = vec![
        RankMovement {
            ticker: "A".to_string(),
            name: "A".to_string(),
            rank_from: 5,
            rank_to: 5,
            rank_change: 0,
        },
        RankMovement {
            ticker: "B".to_string(),
            name: "B".to_string(),
            rank_from: 3,
            rank_to: 1,
            rank_change: 2,
        },
    ];

    let improvements = get_top_rank_improvements(&data, 10);
    let declines = get_top_rank_declines(&data, 10);

    assert_eq!(improvements.len(), 1); // Only B improved
    assert_eq!(declines.len(), 0); // No declines
}

// ==================== Donut Chart Tests ====================

#[test]
fn test_calculate_others_basic() {
    let data: Vec<ChartDataPoint> = (0..15)
        .map(|i| {
            ChartDataPoint {
                ticker: format!("T{}", i),
                name: format!("Company {}", i),
                value: (15 - i) as f64 * 1_000_000_000.0, // Descending values
            }
        })
        .collect();

    // Top 10 + "Others"
    let others = calculate_others(&data, 10);

    // Sum of items 10-14
    let expected: f64 = (0..5).map(|i| (5 - i) as f64 * 1_000_000_000.0).sum();
    assert!((others - expected).abs() < 0.01);
}

#[test]
fn test_calculate_others_fewer_than_threshold() {
    let data: Vec<ChartDataPoint> = (0..5)
        .map(|i| ChartDataPoint {
            ticker: format!("T{}", i),
            name: format!("Company {}", i),
            value: 1_000_000_000.0,
        })
        .collect();

    let others = calculate_others(&data, 10);

    assert_eq!(others, 0.0); // No "others" when less than threshold
}

#[test]
fn test_calculate_others_exact_threshold() {
    let data: Vec<ChartDataPoint> = (0..10)
        .map(|i| ChartDataPoint {
            ticker: format!("T{}", i),
            name: format!("Company {}", i),
            value: 1_000_000_000.0,
        })
        .collect();

    let others = calculate_others(&data, 10);

    assert_eq!(others, 0.0); // Exactly 10 items, no "others"
}

// ==================== Format Tests ====================

#[test]
fn test_format_market_cap_trillions() {
    assert_eq!(format_market_cap(1_500_000_000_000.0), "$1.5T");
    assert_eq!(format_market_cap(3_000_000_000_000.0), "$3.0T");
}

#[test]
fn test_format_market_cap_billions() {
    assert_eq!(format_market_cap(500_000_000_000.0), "$500.0B");
    assert_eq!(format_market_cap(50_000_000_000.0), "$50.0B");
}

#[test]
fn test_format_market_cap_millions() {
    assert_eq!(format_market_cap(500_000_000.0), "$500.0M");
    assert_eq!(format_market_cap(50_000_000.0), "$50.0M");
}

#[test]
fn test_format_market_cap_small() {
    assert_eq!(format_market_cap(500_000.0), "$500000");
    assert_eq!(format_market_cap(1000.0), "$1000");
}

#[test]
fn test_format_market_cap_boundary() {
    // Just under trillion
    assert_eq!(format_market_cap(999_999_999_999.0), "$1000.0B");

    // Just under billion
    assert_eq!(format_market_cap(999_999_999.0), "$1000.0M");
}

// ==================== Color Selection Tests ====================

#[test]
fn test_color_index_basic() {
    assert_eq!(get_color_index(0, 10), 0);
    assert_eq!(get_color_index(5, 10), 5);
    assert_eq!(get_color_index(9, 10), 9);
}

#[test]
fn test_color_index_wraps() {
    assert_eq!(get_color_index(10, 10), 0); // Wraps to first
    assert_eq!(get_color_index(15, 10), 5);
    assert_eq!(get_color_index(23, 10), 3);
}

#[test]
fn test_color_index_single_color() {
    // Edge case: only 1 color available
    assert_eq!(get_color_index(0, 1), 0);
    assert_eq!(get_color_index(5, 1), 0);
    assert_eq!(get_color_index(100, 1), 0);
}

// ==================== Data Validation Tests ====================

#[test]
fn test_chart_data_point_ordering() {
    let mut data = vec![
        ChartDataPoint {
            ticker: "C".to_string(),
            name: "C Inc".to_string(),
            value: 100.0,
        },
        ChartDataPoint {
            ticker: "A".to_string(),
            name: "A Inc".to_string(),
            value: 300.0,
        },
        ChartDataPoint {
            ticker: "B".to_string(),
            name: "B Inc".to_string(),
            value: 200.0,
        },
    ];

    // Sort by value descending (for chart display)
    data.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());

    assert_eq!(data[0].ticker, "A");
    assert_eq!(data[1].ticker, "B");
    assert_eq!(data[2].ticker, "C");
}

#[test]
fn test_empty_data_handling() {
    let data: Vec<GainerLoser> = vec![];

    let gainers = get_top_gainers(&data, 10);
    let losers = get_top_losers(&data, 10);

    assert!(gainers.is_empty());
    assert!(losers.is_empty());
}

#[test]
fn test_all_gainers_no_losers() {
    let data = vec![
        GainerLoser {
            ticker: "A".to_string(),
            name: "A".to_string(),
            percentage_change: 10.0,
            absolute_change: 100.0,
        },
        GainerLoser {
            ticker: "B".to_string(),
            name: "B".to_string(),
            percentage_change: 20.0,
            absolute_change: 200.0,
        },
    ];

    let losers = get_top_losers(&data, 10);

    assert!(losers.is_empty());
}

#[test]
fn test_all_losers_no_gainers() {
    let data = vec![
        GainerLoser {
            ticker: "A".to_string(),
            name: "A".to_string(),
            percentage_change: -10.0,
            absolute_change: -100.0,
        },
        GainerLoser {
            ticker: "B".to_string(),
            name: "B".to_string(),
            percentage_change: -20.0,
            absolute_change: -200.0,
        },
    ];

    let gainers = get_top_gainers(&data, 10);

    assert!(gainers.is_empty());
}

// ==================== Realistic Data Tests ====================

#[test]
fn test_realistic_market_data() {
    let data = vec![
        GainerLoser {
            ticker: "NVDA".to_string(),
            name: "NVIDIA".to_string(),
            percentage_change: 45.2,
            absolute_change: 500_000_000_000.0,
        },
        GainerLoser {
            ticker: "TSLA".to_string(),
            name: "Tesla".to_string(),
            percentage_change: -12.5,
            absolute_change: -100_000_000_000.0,
        },
        GainerLoser {
            ticker: "AAPL".to_string(),
            name: "Apple".to_string(),
            percentage_change: 8.3,
            absolute_change: 250_000_000_000.0,
        },
        GainerLoser {
            ticker: "META".to_string(),
            name: "Meta".to_string(),
            percentage_change: 22.1,
            absolute_change: 200_000_000_000.0,
        },
        GainerLoser {
            ticker: "AMZN".to_string(),
            name: "Amazon".to_string(),
            percentage_change: -5.2,
            absolute_change: -75_000_000_000.0,
        },
    ];

    let top_gainers = get_top_gainers(&data, 3);
    let top_losers = get_top_losers(&data, 2);

    // Top gainers should be NVDA, META, AAPL
    assert_eq!(top_gainers.len(), 3);
    assert_eq!(top_gainers[0].ticker, "NVDA");
    assert_eq!(top_gainers[1].ticker, "META");
    assert_eq!(top_gainers[2].ticker, "AAPL");

    // Top losers should be TSLA, AMZN
    assert_eq!(top_losers.len(), 2);
    assert_eq!(top_losers[0].ticker, "TSLA"); // -12.5%
    assert_eq!(top_losers[1].ticker, "AMZN"); // -5.2%
}

#[test]
fn test_market_cap_distribution_chart_data() {
    // Simulate top 10 + others for donut chart
    let mut data: Vec<ChartDataPoint> = vec![
        ChartDataPoint {
            ticker: "AAPL".to_string(),
            name: "Apple".to_string(),
            value: 3_000_000_000_000.0,
        },
        ChartDataPoint {
            ticker: "MSFT".to_string(),
            name: "Microsoft".to_string(),
            value: 2_800_000_000_000.0,
        },
        ChartDataPoint {
            ticker: "GOOGL".to_string(),
            name: "Alphabet".to_string(),
            value: 1_700_000_000_000.0,
        },
        ChartDataPoint {
            ticker: "AMZN".to_string(),
            name: "Amazon".to_string(),
            value: 1_500_000_000_000.0,
        },
        ChartDataPoint {
            ticker: "NVDA".to_string(),
            name: "NVIDIA".to_string(),
            value: 1_200_000_000_000.0,
        },
    ];

    // Add more to create "others"
    for i in 5..15 {
        data.push(ChartDataPoint {
            ticker: format!("T{}", i),
            name: format!("Company {}", i),
            value: (100 - i * 5) as f64 * 1_000_000_000.0,
        });
    }

    // Calculate total
    let total: f64 = data.iter().map(|d| d.value).sum();

    // Calculate top 10 percentage
    let top10_sum: f64 = data.iter().take(10).map(|d| d.value).sum();
    let top10_pct = (top10_sum / total) * 100.0;

    // Top 10 should represent majority of market cap
    assert!(
        top10_pct > 90.0,
        "Top 10 should be > 90% of total, got {:.1}%",
        top10_pct
    );

    // Others category
    let others = calculate_others(&data, 10);
    let others_pct = (others / total) * 100.0;
    assert!(
        others_pct < 10.0,
        "Others should be < 10% of total, got {:.1}%",
        others_pct
    );
}
