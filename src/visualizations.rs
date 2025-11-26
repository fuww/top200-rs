// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use csv::Reader;
use plotters::prelude::*;
use serde::Deserialize;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ComparisonRecord {
    #[serde(rename = "Ticker")]
    ticker: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Market Cap From (USD)")]
    market_cap_from: Option<String>,
    #[serde(rename = "Market Cap To (USD)")]
    market_cap_to: Option<String>,
    #[serde(rename = "Absolute Change (USD)")]
    _absolute_change: Option<String>,
    #[serde(rename = "Percentage Change (%)")]
    percentage_change: Option<String>,
    #[serde(rename = "Rank From")]
    rank_from: Option<String>,
    #[serde(rename = "Rank To")]
    rank_to: Option<String>,
    #[serde(rename = "Rank Change")]
    rank_change: Option<String>,
    #[serde(rename = "Market Share From (%)")]
    _market_share_from: Option<String>,
    #[serde(rename = "Market Share To (%)")]
    _market_share_to: Option<String>,
}

// Professional color palette
const COLOR_EMERALD: RGBColor = RGBColor(16, 185, 129);
const COLOR_ROSE: RGBColor = RGBColor(244, 63, 94);
const COLOR_BLUE: RGBColor = RGBColor(59, 130, 246);
const COLOR_AMBER: RGBColor = RGBColor(245, 158, 11);
const COLOR_TEAL: RGBColor = RGBColor(20, 184, 166);
const COLOR_CORAL: RGBColor = RGBColor(251, 113, 133);
const COLOR_PURPLE: RGBColor = RGBColor(139, 92, 246);
const COLOR_PINK: RGBColor = RGBColor(236, 72, 153);
const COLOR_LIME: RGBColor = RGBColor(132, 204, 22);
const COLOR_ORANGE: RGBColor = RGBColor(249, 115, 22);
const COLOR_SLATE: RGBColor = RGBColor(100, 116, 139);
const COLOR_GRAY_LIGHT: RGBColor = RGBColor(243, 244, 246);

const CHART_COLORS: [RGBColor; 10] = [
    COLOR_BLUE,
    COLOR_EMERALD,
    COLOR_AMBER,
    COLOR_ROSE,
    COLOR_PURPLE,
    COLOR_PINK,
    COLOR_TEAL,
    COLOR_ORANGE,
    COLOR_LIME,
    COLOR_SLATE,
];

/// Find the comparison CSV file for the given dates
fn find_comparison_csv(from_date: &str, to_date: &str) -> Result<String> {
    let output_dir = Path::new("output");
    let pattern = format!("comparison_{}_to_{}_", from_date, to_date);

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
            "No comparison CSV found for {} to {}. Please run 'compare-market-caps' first.",
            from_date,
            to_date
        );
    }

    matching_files.sort();
    let selected_file = matching_files.last().unwrap();

    Ok(format!("output/{}", selected_file))
}

/// Read comparison data from CSV
fn read_comparison_data(csv_path: &str) -> Result<Vec<ComparisonRecord>> {
    let file =
        File::open(csv_path).with_context(|| format!("Failed to open CSV file: {}", csv_path))?;

    let mut reader = Reader::from_reader(file);
    let mut records = Vec::new();

    for result in reader.deserialize() {
        let record: ComparisonRecord = result?;
        records.push(record);
    }

    Ok(records)
}

/// Parse percentage string to f64
fn parse_percentage(s: &Option<String>) -> Option<f64> {
    s.as_ref()?.parse::<f64>().ok()
}

/// Safely truncate a string to a maximum number of characters, respecting UTF-8 boundaries
fn truncate_string(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Parse USD amount string to f64
fn parse_usd_amount(s: &Option<String>) -> Option<f64> {
    s.as_ref()?.parse::<f64>().ok()
}

/// Create top gainers and losers bar chart
fn create_gainers_losers_chart(
    records: &[ComparisonRecord],
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    // Filter and sort for top gainers
    let mut gainers: Vec<_> = records
        .iter()
        .filter_map(|r| {
            let pct = parse_percentage(&r.percentage_change)?;
            if pct > 0.0 {
                Some((r.name.clone(), pct))
            } else {
                None
            }
        })
        .collect();
    gainers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    gainers.truncate(10);

    // Filter and sort for top losers
    let mut losers: Vec<_> = records
        .iter()
        .filter_map(|r| {
            let pct = parse_percentage(&r.percentage_change)?;
            if pct < 0.0 {
                Some((r.name.clone(), pct))
            } else {
                None
            }
        })
        .collect();
    losers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    losers.truncate(10);

    // Create the chart
    let filename = format!(
        "output/comparison_{}_to_{}_gainers_losers.svg",
        from_date, to_date
    );
    let root = SVGBackend::new(&filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("Top Gainers and Losers: {} to {}", from_date, to_date),
            ("sans-serif", 32).into_font().color(&BLACK),
        )
        .margin(20)
        .x_label_area_size(150)
        .y_label_area_size(50)
        .build_cartesian_2d(-100f64..250f64, 0usize..20usize)?;

    chart
        .configure_mesh()
        .x_desc("Percentage Change (%)")
        .y_desc("")
        .x_label_formatter(&|x| format!("{:.0}%", x))
        .y_label_formatter(&|_| "".to_string())
        .axis_desc_style(("sans-serif", 16))
        .draw()?;

    // Draw gainers (green gradient)
    for (i, (name, pct)) in gainers.iter().enumerate() {
        let y = 19 - i;
        let y_coord = y as i32;
        let color = RGBColor(
            16 + (i * 10) as u8,
            185 - (i * 5) as u8,
            129 - (i * 5) as u8,
        );

        chart.draw_series(std::iter::once(Rectangle::new(
            [(0.0, y), (*pct, y.saturating_sub(1))],
            color.filled(),
        )))?;

        // Add label
        let label_name = truncate_string(name, 30);

        root.draw_text(
            &label_name,
            &TextStyle::from(("sans-serif", 14).into_font()),
            (50, 80 + y_coord * 35),
        )?;

        // Add value label
        root.draw_text(
            &format!("+{:.1}%", pct),
            &TextStyle::from(("sans-serif", 12).into_font()).color(&COLOR_EMERALD),
            (1050, 80 + y_coord * 35),
        )?;
    }

    // Draw losers (red gradient)
    for (i, (name, pct)) in losers.iter().enumerate() {
        let y = 9 - i;
        let y_coord = y as i32;
        // Use saturating arithmetic to prevent u8 underflow
        let color = RGBColor(
            244u8.saturating_sub((i * 5) as u8),
            63u8.saturating_add((i * 5) as u8),
            94u8.saturating_add((i * 5) as u8),
        );

        chart.draw_series(std::iter::once(Rectangle::new(
            [(0.0, y), (*pct, y.saturating_sub(1))],
            color.filled(),
        )))?;

        // Add label
        let label_name = truncate_string(name, 30);

        root.draw_text(
            &label_name,
            &TextStyle::from(("sans-serif", 14).into_font()),
            (50, 440 + (9 - y_coord) * 35),
        )?;

        // Add value label
        root.draw_text(
            &format!("{:.1}%", pct),
            &TextStyle::from(("sans-serif", 12).into_font()).color(&COLOR_ROSE),
            (1050, 440 + (9 - y_coord) * 35),
        )?;
    }

    // Add dividing line
    chart.draw_series(std::iter::once(PathElement::new(
        vec![(0.0, 10), (0.0, 10)],
        BLACK.stroke_width(2),
    )))?;

    root.present()?;
    println!("✅ Generated gainers/losers chart: {}", filename);

    Ok(())
}

/// Create market cap distribution donut chart
fn create_market_distribution_chart(
    records: &[ComparisonRecord],
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    // Get top 10 companies by market cap
    let mut companies: Vec<_> = records
        .iter()
        .filter_map(|r| {
            let market_cap = parse_usd_amount(&r.market_cap_to)?;
            Some((r.ticker.clone(), r.name.clone(), market_cap))
        })
        .collect();
    companies.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    let total_market_cap: f64 = companies.iter().map(|c| c.2).sum();
    let top_10 = companies.iter().take(10).cloned().collect::<Vec<_>>();
    let top_10_sum: f64 = top_10.iter().map(|c| c.2).sum();
    let others = total_market_cap - top_10_sum;

    // Create the chart
    let filename = format!(
        "output/comparison_{}_to_{}_market_distribution.svg",
        from_date, to_date
    );
    let root = SVGBackend::new(&filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Title
    root.draw_text(
        &format!("Market Cap Distribution: {}", to_date),
        &TextStyle::from(("sans-serif", 32).into_font()).color(&BLACK),
        (400, 30),
    )?;

    // Draw donut chart
    let center = (400, 400);
    let outer_radius = 250.0;
    let inner_radius = 120.0;

    let mut start_angle = -90.0; // Start from top

    for (i, (_ticker, _name, market_cap)) in top_10.iter().enumerate() {
        let percentage = (market_cap / total_market_cap) * 100.0;
        let sweep_angle = (percentage / 100.0) * 360.0;

        // Draw segment
        draw_donut_segment(
            &root,
            center,
            outer_radius,
            inner_radius,
            start_angle,
            sweep_angle,
            CHART_COLORS[i],
        )?;

        start_angle += sweep_angle;
    }

    // Draw "Others" segment
    if others > 0.0 {
        let percentage = (others / total_market_cap) * 100.0;
        let sweep_angle = (percentage / 100.0) * 360.0;

        draw_donut_segment(
            &root,
            center,
            outer_radius,
            inner_radius,
            start_angle,
            sweep_angle,
            COLOR_GRAY_LIGHT,
        )?;
    }

    // Draw legend
    let legend_x = 750;
    let legend_y_start = 150;

    for (i, (ticker, name, market_cap)) in top_10.iter().enumerate() {
        let y = legend_y_start + (i as i32) * 35;

        // Color box
        root.draw(&Rectangle::new(
            [(legend_x, y), (legend_x + 20, y + 20)],
            CHART_COLORS[i].filled(),
        ))?;

        // Company name
        let display_name = truncate_string(name, 25);

        root.draw_text(
            &format!("{} ({})", display_name, ticker),
            &TextStyle::from(("sans-serif", 14).into_font()),
            (legend_x + 30, y + 5),
        )?;

        // Percentage
        let percentage = (market_cap / total_market_cap) * 100.0;
        root.draw_text(
            &format!("{:.1}%", percentage),
            &TextStyle::from(("sans-serif", 12).into_font()).color(&COLOR_SLATE),
            (legend_x + 30, y + 20),
        )?;
    }

    // Add "Others" to legend
    if others > 0.0 {
        let y = legend_y_start + 10 * 35;
        root.draw(&Rectangle::new(
            [(legend_x, y), (legend_x + 20, y + 20)],
            COLOR_GRAY_LIGHT.filled(),
        ))?;

        root.draw_text(
            "Others",
            &TextStyle::from(("sans-serif", 14).into_font()),
            (legend_x + 30, y + 5),
        )?;

        let percentage = (others / total_market_cap) * 100.0;
        root.draw_text(
            &format!("{:.1}%", percentage),
            &TextStyle::from(("sans-serif", 12).into_font()).color(&COLOR_SLATE),
            (legend_x + 30, y + 20),
        )?;
    }

    // Add center text with total
    root.draw_text(
        "Total Market Cap",
        &TextStyle::from(("sans-serif", 16).into_font()).color(&COLOR_SLATE),
        (center.0 - 60, center.1 - 10),
    )?;
    root.draw_text(
        &format!("${:.1}T", total_market_cap / 1_000_000_000_000.0),
        &TextStyle::from(("sans-serif", 24).into_font()).color(&BLACK),
        (center.0 - 40, center.1 + 10),
    )?;

    root.present()?;
    println!("✅ Generated market distribution chart: {}", filename);

    Ok(())
}

/// Draw a donut segment
fn draw_donut_segment(
    root: &DrawingArea<SVGBackend, plotters::coord::Shift>,
    center: (i32, i32),
    outer_radius: f64,
    inner_radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    color: RGBColor,
) -> Result<()> {
    let num_points = 100;
    let mut points = Vec::new();

    // Outer arc
    for i in 0..=num_points {
        let angle = start_angle + (sweep_angle * i as f64 / num_points as f64);
        let rad = angle.to_radians();
        let x = center.0 + (outer_radius * rad.cos()) as i32;
        let y = center.1 + (outer_radius * rad.sin()) as i32;
        points.push((x, y));
    }

    // Inner arc (reverse)
    for i in (0..=num_points).rev() {
        let angle = start_angle + (sweep_angle * i as f64 / num_points as f64);
        let rad = angle.to_radians();
        let x = center.0 + (inner_radius * rad.cos()) as i32;
        let y = center.1 + (inner_radius * rad.sin()) as i32;
        points.push((x, y));
    }

    root.draw(&Polygon::new(points, color.filled()))?;

    Ok(())
}

/// Create rank movement chart
fn create_rank_movement_chart(
    records: &[ComparisonRecord],
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    // Parse rank changes
    let mut rank_changes: Vec<_> = records
        .iter()
        .filter_map(|r| {
            let rank_change_str = r.rank_change.as_ref()?;
            if rank_change_str == "NA" {
                return None;
            }
            let rank_change = rank_change_str
                .trim_start_matches('+')
                .parse::<i32>()
                .ok()?;
            if rank_change != 0 {
                Some((
                    r.name.clone(),
                    rank_change,
                    r.rank_from.clone(),
                    r.rank_to.clone(),
                ))
            } else {
                None
            }
        })
        .collect();

    // Get top 10 improvements and declines
    rank_changes.sort_by(|a, b| b.1.cmp(&a.1));
    let improvements = rank_changes
        .iter()
        .filter(|r| r.1 > 0)
        .take(10)
        .cloned()
        .collect::<Vec<_>>();

    rank_changes.sort_by(|a, b| a.1.cmp(&b.1));
    let declines = rank_changes
        .iter()
        .filter(|r| r.1 < 0)
        .take(10)
        .cloned()
        .collect::<Vec<_>>();

    // Create the chart
    let filename = format!(
        "output/comparison_{}_to_{}_rank_movements.svg",
        from_date, to_date
    );
    let root = SVGBackend::new(&filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Title
    root.draw_text(
        &format!("Rank Movements: {} to {}", from_date, to_date),
        &TextStyle::from(("sans-serif", 32).into_font()).color(&BLACK),
        (350, 30),
    )?;

    // Draw improvements
    root.draw_text(
        "Biggest Rank Improvements",
        &TextStyle::from(("sans-serif", 20).into_font()).color(&COLOR_TEAL),
        (150, 100),
    )?;

    for (i, (name, change, from, to)) in improvements.iter().enumerate() {
        let y = 140 + i * 30;
        let bar_width = (*change as f64 * 50.0) as i32;

        // Draw bar
        root.draw(&Rectangle::new(
            [(200, y as i32), (200 + bar_width, (y + 20) as i32)],
            COLOR_TEAL.filled(),
        ))?;

        // Company name
        let display_name = truncate_string(name, 25);

        root.draw_text(
            &display_name,
            &TextStyle::from(("sans-serif", 12).into_font()),
            (10, y as i32),
        )?;

        // Change value
        root.draw_text(
            &format!(
                "+{} (#{} → #{})",
                change,
                from.as_ref().unwrap_or(&"NA".to_string()),
                to.as_ref().unwrap_or(&"NA".to_string())
            ),
            &TextStyle::from(("sans-serif", 11).into_font()).color(&COLOR_TEAL),
            (210 + bar_width, y as i32 + 5),
        )?;
    }

    // Draw declines
    root.draw_text(
        "Biggest Rank Declines",
        &TextStyle::from(("sans-serif", 20).into_font()).color(&COLOR_CORAL),
        (150, 450),
    )?;

    for (i, (name, change, from, to)) in declines.iter().enumerate() {
        let y = 490 + i * 30;
        let bar_width = (change.abs() as f64 * 50.0) as i32;

        // Draw bar
        root.draw(&Rectangle::new(
            [(200, y as i32), (200 + bar_width, (y + 20) as i32)],
            COLOR_CORAL.filled(),
        ))?;

        // Company name
        let display_name = truncate_string(name, 25);

        root.draw_text(
            &display_name,
            &TextStyle::from(("sans-serif", 12).into_font()),
            (10, y as i32),
        )?;

        // Change value
        root.draw_text(
            &format!(
                "{} (#{} → #{})",
                change,
                from.as_ref().unwrap_or(&"NA".to_string()),
                to.as_ref().unwrap_or(&"NA".to_string())
            ),
            &TextStyle::from(("sans-serif", 11).into_font()).color(&COLOR_CORAL),
            (210 + bar_width, y as i32 + 5),
        )?;
    }

    root.present()?;
    println!("✅ Generated rank movements chart: {}", filename);

    Ok(())
}

/// Create market summary dashboard
fn create_summary_dashboard(
    records: &[ComparisonRecord],
    from_date: &str,
    to_date: &str,
) -> Result<()> {
    // Calculate metrics
    let total_from: f64 = records
        .iter()
        .filter_map(|r| parse_usd_amount(&r.market_cap_from))
        .sum();

    let total_to: f64 = records
        .iter()
        .filter_map(|r| parse_usd_amount(&r.market_cap_to))
        .sum();

    let total_change = total_to - total_from;
    let total_pct_change = if total_from > 0.0 {
        (total_change / total_from) * 100.0
    } else {
        0.0
    };

    let gainers = records
        .iter()
        .filter(|r| parse_percentage(&r.percentage_change).unwrap_or(0.0) > 0.0)
        .count();

    let losers = records
        .iter()
        .filter(|r| parse_percentage(&r.percentage_change).unwrap_or(0.0) < 0.0)
        .count();

    let unchanged = records.len() - gainers - losers;

    // Create the dashboard
    let filename = format!(
        "output/comparison_{}_to_{}_summary_dashboard.svg",
        from_date, to_date
    );
    let root = SVGBackend::new(&filename, (1200, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    // Title
    root.draw_text(
        &format!("Market Summary: {} to {}", from_date, to_date),
        &TextStyle::from(("sans-serif", 36).into_font()).color(&BLACK),
        (300, 40),
    )?;

    // Main metric box
    let metric_color = if total_change >= 0.0 {
        COLOR_EMERALD
    } else {
        COLOR_ROSE
    };
    let arrow = if total_change >= 0.0 { "↑" } else { "↓" };

    // Background box
    root.draw(&Rectangle::new(
        [(100, 120), (500, 280)],
        COLOR_GRAY_LIGHT.filled(),
    ))?;

    root.draw_text(
        "Total Market Cap Change",
        &TextStyle::from(("sans-serif", 18).into_font()).color(&COLOR_SLATE),
        (220, 140),
    )?;

    root.draw_text(
        &format!("{} ${:.2}B", arrow, total_change.abs() / 1_000_000_000.0),
        &TextStyle::from(("sans-serif", 48).into_font()).color(&metric_color),
        (180, 190),
    )?;

    root.draw_text(
        &format!("{:.2}%", total_pct_change),
        &TextStyle::from(("sans-serif", 32).into_font()).color(&metric_color),
        (250, 240),
    )?;

    // From and To values
    root.draw(&Rectangle::new(
        [(600, 120), (1100, 280)],
        COLOR_GRAY_LIGHT.filled(),
    ))?;

    root.draw_text(
        &format!("{}: ${:.2}T", from_date, total_from / 1_000_000_000_000.0),
        &TextStyle::from(("sans-serif", 20).into_font()),
        (650, 160),
    )?;

    root.draw_text(
        &format!("{}: ${:.2}T", to_date, total_to / 1_000_000_000_000.0),
        &TextStyle::from(("sans-serif", 20).into_font()),
        (650, 200),
    )?;

    root.draw_text(
        &format!("Companies Analyzed: {}", records.len()),
        &TextStyle::from(("sans-serif", 16).into_font()).color(&COLOR_SLATE),
        (650, 240),
    )?;

    // Gainers vs Losers pie chart
    let pie_center = (300, 500);
    let pie_radius = 120.0;

    root.draw_text(
        "Market Movement Distribution",
        &TextStyle::from(("sans-serif", 20).into_font()),
        (180, 350),
    )?;

    // Calculate angles
    let total_companies = gainers + losers + unchanged;
    let gainers_angle = (gainers as f64 / total_companies as f64) * 360.0;
    let losers_angle = (losers as f64 / total_companies as f64) * 360.0;

    // Draw pie segments
    draw_pie_segment(
        &root,
        pie_center,
        pie_radius,
        -90.0,
        gainers_angle,
        COLOR_EMERALD,
    )?;
    draw_pie_segment(
        &root,
        pie_center,
        pie_radius,
        -90.0 + gainers_angle,
        losers_angle,
        COLOR_ROSE,
    )?;
    draw_pie_segment(
        &root,
        pie_center,
        pie_radius,
        -90.0 + gainers_angle + losers_angle,
        360.0 - gainers_angle - losers_angle,
        COLOR_SLATE,
    )?;

    // Legend for pie chart
    root.draw(&Rectangle::new(
        [(500, 450), (520, 470)],
        COLOR_EMERALD.filled(),
    ))?;
    root.draw_text(
        &format!(
            "Gainers: {} ({:.1}%)",
            gainers,
            (gainers as f64 / total_companies as f64) * 100.0
        ),
        &TextStyle::from(("sans-serif", 14).into_font()),
        (530, 455),
    )?;

    root.draw(&Rectangle::new(
        [(500, 490), (520, 510)],
        COLOR_ROSE.filled(),
    ))?;
    root.draw_text(
        &format!(
            "Losers: {} ({:.1}%)",
            losers,
            (losers as f64 / total_companies as f64) * 100.0
        ),
        &TextStyle::from(("sans-serif", 14).into_font()),
        (530, 495),
    )?;

    root.draw(&Rectangle::new(
        [(500, 530), (520, 550)],
        COLOR_SLATE.filled(),
    ))?;
    root.draw_text(
        &format!(
            "Unchanged: {} ({:.1}%)",
            unchanged,
            (unchanged as f64 / total_companies as f64) * 100.0
        ),
        &TextStyle::from(("sans-serif", 14).into_font()),
        (530, 535),
    )?;

    // Key statistics box
    root.draw(&Rectangle::new(
        [(750, 400), (1100, 620)],
        COLOR_GRAY_LIGHT.filled(),
    ))?;

    root.draw_text(
        "Key Statistics",
        &TextStyle::from(("sans-serif", 20).into_font()),
        (850, 420),
    )?;

    // Calculate average change (avoid division by zero)
    let avg_change: f64 = if records.is_empty() {
        0.0
    } else {
        records
            .iter()
            .filter_map(|r| parse_percentage(&r.percentage_change))
            .sum::<f64>()
            / records.len() as f64
    };

    root.draw_text(
        &format!("Average Change: {:.2}%", avg_change),
        &TextStyle::from(("sans-serif", 14).into_font()),
        (780, 460),
    )?;

    // Find biggest gainer and loser
    let biggest_gainer = records.iter().max_by(|a, b| {
        parse_percentage(&a.percentage_change)
            .unwrap_or(0.0)
            .partial_cmp(&parse_percentage(&b.percentage_change).unwrap_or(0.0))
            .unwrap()
    });

    let biggest_loser = records.iter().min_by(|a, b| {
        parse_percentage(&a.percentage_change)
            .unwrap_or(0.0)
            .partial_cmp(&parse_percentage(&b.percentage_change).unwrap_or(0.0))
            .unwrap()
    });

    if let Some(gainer) = biggest_gainer {
        let name = truncate_string(&gainer.name, 20);
        root.draw_text(
            &format!("Top Gainer: {}", name),
            &TextStyle::from(("sans-serif", 14).into_font()),
            (780, 490),
        )?;
        root.draw_text(
            &format!(
                "  +{:.1}%",
                parse_percentage(&gainer.percentage_change).unwrap_or(0.0)
            ),
            &TextStyle::from(("sans-serif", 14).into_font()).color(&COLOR_EMERALD),
            (780, 510),
        )?;
    }

    if let Some(loser) = biggest_loser {
        let name = truncate_string(&loser.name, 20);
        root.draw_text(
            &format!("Top Loser: {}", name),
            &TextStyle::from(("sans-serif", 14).into_font()),
            (780, 540),
        )?;
        root.draw_text(
            &format!(
                "  {:.1}%",
                parse_percentage(&loser.percentage_change).unwrap_or(0.0)
            ),
            &TextStyle::from(("sans-serif", 14).into_font()).color(&COLOR_ROSE),
            (780, 560),
        )?;
    }

    // Footer
    root.draw_text(
        &format!(
            "Generated on {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
        &TextStyle::from(("sans-serif", 10).into_font()).color(&COLOR_SLATE),
        (450, 750),
    )?;

    root.present()?;
    println!("✅ Generated summary dashboard: {}", filename);

    Ok(())
}

/// Draw a pie segment
fn draw_pie_segment(
    root: &DrawingArea<SVGBackend, plotters::coord::Shift>,
    center: (i32, i32),
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    color: RGBColor,
) -> Result<()> {
    let num_points = 100;
    let mut points = Vec::new();

    points.push(center);

    for i in 0..=num_points {
        let angle = start_angle + (sweep_angle * i as f64 / num_points as f64);
        let rad = angle.to_radians();
        let x = center.0 + (radius * rad.cos()) as i32;
        let y = center.1 + (radius * rad.sin()) as i32;
        points.push((x, y));
    }

    root.draw(&Polygon::new(points, color.filled()))?;

    Ok(())
}

/// Main function to generate all charts
pub async fn generate_all_charts(from_date: &str, to_date: &str) -> Result<()> {
    println!(
        "Generating visualization charts for {} to {}",
        from_date, to_date
    );

    // Find and read the comparison CSV
    let csv_path = find_comparison_csv(from_date, to_date)?;
    println!("Reading data from: {}", csv_path);

    let records = read_comparison_data(&csv_path)?;
    println!("Loaded {} companies for visualization", records.len());

    // Generate each chart type
    println!("\nGenerating charts...");

    create_gainers_losers_chart(&records, from_date, to_date)?;
    create_market_distribution_chart(&records, from_date, to_date)?;
    create_rank_movement_chart(&records, from_date, to_date)?;
    create_summary_dashboard(&records, from_date, to_date)?;

    println!("\n✅ All charts generated successfully!");

    Ok(())
}
