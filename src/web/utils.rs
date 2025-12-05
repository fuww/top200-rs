// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::NaiveDate;
use csv::Reader;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata about a comparison between two dates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetadata {
    pub from_date: String,
    pub to_date: String,
    pub timestamp: String,
    pub csv_path: PathBuf,
    pub summary_path: Option<PathBuf>,
    pub chart_paths: Vec<ChartFile>,
}

/// Chart file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartFile {
    pub chart_type: String,
    pub path: PathBuf,
}

/// Comparison data from CSV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonRecord {
    #[serde(rename = "Ticker")]
    pub ticker: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Currency")]
    pub currency: String,
    #[serde(rename = "Market Cap From")]
    pub market_cap_from: String,
    #[serde(rename = "Market Cap To")]
    pub market_cap_to: String,
    #[serde(rename = "Absolute Change")]
    pub absolute_change: String,
    #[serde(rename = "Percentage Change (%)")]
    pub percentage_change: String,
    #[serde(rename = "Rank From")]
    pub rank_from: String,
    #[serde(rename = "Rank To")]
    pub rank_to: String,
    #[serde(rename = "Rank Change")]
    pub rank_change: String,
    #[serde(rename = "Market Share From (%)")]
    pub market_share_from: String,
    #[serde(rename = "Market Share To (%)")]
    pub market_share_to: String,
}

/// Scan the output directory for comparison files
pub fn list_comparisons() -> Result<Vec<ComparisonMetadata>> {
    let output_dir = Path::new("output");

    if !output_dir.exists() {
        return Ok(Vec::new());
    }

    let mut comparisons: Vec<ComparisonMetadata> = Vec::new();
    let entries = fs::read_dir(output_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Look for comparison CSV files
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with("comparison_") && filename.ends_with(".csv") {
                // Parse filename: comparison_{from}_to_{to}_{timestamp}.csv
                if let Some(metadata) = parse_comparison_filename(filename, &path) {
                    comparisons.push(metadata);
                }
            }
        }
    }

    // Sort by date (most recent first)
    comparisons.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(comparisons)
}

/// Parse comparison filename to extract metadata
fn parse_comparison_filename(filename: &str, csv_path: &Path) -> Option<ComparisonMetadata> {
    // Expected format: comparison_{from}_to_{to}_{timestamp}.csv
    let parts: Vec<&str> = filename.strip_prefix("comparison_")?.strip_suffix(".csv")?.split('_').collect();

    if parts.len() < 4 {
        return None;
    }

    // Find "to" separator
    let to_index = parts.iter().position(|&p| p == "to")?;

    let from_date = parts[..to_index].join("-");
    let to_date = parts[to_index + 1..parts.len() - 1].join("-");
    let timestamp = parts.last()?.to_string();

    // Find associated files
    let base_pattern = format!("comparison_{}_to_{}", from_date, to_date);
    let summary_path = find_file_with_pattern(&base_pattern, "_summary_", ".md");
    let chart_paths = find_chart_files(&base_pattern);

    Some(ComparisonMetadata {
        from_date,
        to_date,
        timestamp,
        csv_path: csv_path.to_path_buf(),
        summary_path,
        chart_paths,
    })
}

/// Find a file matching a pattern
fn find_file_with_pattern(base: &str, middle: &str, ext: &str) -> Option<PathBuf> {
    let output_dir = Path::new("output");
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with(base) && filename.contains(middle) && filename.ends_with(ext) {
                    return Some(entry.path());
                }
            }
        }
    }
    None
}

/// Find all chart files for a comparison
fn find_chart_files(base_pattern: &str) -> Vec<ChartFile> {
    let output_dir = Path::new("output");
    let mut charts = Vec::new();

    let chart_types = vec![
        "gainers_losers",
        "market_distribution",
        "rank_movements",
        "summary_dashboard",
    ];

    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with(base_pattern) && filename.ends_with(".svg") {
                    // Determine chart type from filename
                    for chart_type in &chart_types {
                        if filename.contains(chart_type) {
                            charts.push(ChartFile {
                                chart_type: chart_type.to_string(),
                                path: entry.path(),
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    charts
}

/// Read and parse a comparison CSV file
pub fn read_comparison_csv(path: &Path) -> Result<Vec<ComparisonRecord>> {
    let file = fs::File::open(path)
        .with_context(|| format!("Failed to open comparison file: {}", path.display()))?;

    let mut reader = Reader::from_reader(file);
    let mut records = Vec::new();

    for result in reader.deserialize() {
        let record: ComparisonRecord = result?;
        records.push(record);
    }

    Ok(records)
}

/// Read summary markdown file
pub fn read_summary_markdown(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read summary file: {}", path.display()))
}

/// Read chart SVG file
pub fn read_chart_svg(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("Failed to read chart file: {}", path.display()))
}

/// Parse a date string (YYYY-MM-DD)
pub fn parse_date(date_str: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .with_context(|| format!("Invalid date format: {}", date_str))
}

/// Format a date as YYYY-MM-DD
pub fn format_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_comparison_filename() {
        let path = Path::new("output/comparison_2025-01-01_to_2025-02-01_20250201_120000.csv");
        let filename = "comparison_2025-01-01_to_2025-02-01_20250201_120000.csv";

        let metadata = parse_comparison_filename(filename, path);
        assert!(metadata.is_some());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.from_date, "2025-01-01");
        assert_eq!(metadata.to_date, "2025-02-01");
        assert_eq!(metadata.timestamp, "120000");
    }
}
