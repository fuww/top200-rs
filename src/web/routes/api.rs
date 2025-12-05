// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use crate::web::{state::AppState, utils};

/// List all available comparisons
pub async fn list_comparisons(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let comparisons = utils::list_comparisons().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "comparisons": comparisons
    })))
}

/// Get comparison data for specific dates
pub async fn get_comparison(
    State(_state): State<AppState>,
    Path((from_date, to_date)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Find the comparison file
    let comparisons = utils::list_comparisons().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let comparison = comparisons
        .iter()
        .find(|c| c.from_date == from_date && c.to_date == to_date)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Read comparison data
    let records = utils::read_comparison_csv(&comparison.csv_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Read summary if available
    let summary = comparison
        .summary_path
        .as_ref()
        .and_then(|p| utils::read_summary_markdown(p).ok());

    Ok(Json(json!({
        "metadata": comparison,
        "records": records,
        "summary": summary
    })))
}

/// Get a specific chart for a comparison
pub async fn get_chart(
    State(_state): State<AppState>,
    Path((from_date, to_date, chart_type)): Path<(String, String, String)>,
) -> Result<Response, StatusCode> {
    // Find the comparison file
    let comparisons = utils::list_comparisons().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let comparison = comparisons
        .iter()
        .find(|c| c.from_date == from_date && c.to_date == to_date)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Find the chart file
    let chart = comparison
        .chart_paths
        .iter()
        .find(|c| c.chart_type == chart_type)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Read the SVG file
    let svg_content =
        utils::read_chart_svg(&chart.path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::OK,
        [("Content-Type", "image/svg+xml")],
        svg_content,
    )
        .into_response())
}

// ============================================================================
// Market Cap Snapshot API Endpoints
// ============================================================================

/// List all available market cap snapshots
pub async fn list_market_caps(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let snapshots = utils::list_market_caps().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "snapshots": snapshots
    })))
}

/// Get market cap data for a specific date
pub async fn get_market_cap(
    State(_state): State<AppState>,
    Path(date): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Find the market cap file for the date
    let snapshots = utils::list_market_caps().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let snapshot = snapshots
        .iter()
        .find(|s| s.date == date)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Read market cap data
    let records = utils::read_marketcap_csv(&snapshot.csv_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "metadata": snapshot,
        "records": records
    })))
}
