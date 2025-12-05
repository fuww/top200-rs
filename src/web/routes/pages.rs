// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};

use crate::web::{state::AppState, utils};

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    title: String,
}

/// Dashboard page handler
pub async fn dashboard(State(_state): State<AppState>) -> Html<String> {
    let template = DashboardTemplate {
        title: "Dashboard".to_string(),
    };
    Html(template.render().unwrap())
}

#[derive(Template)]
#[template(path = "comparisons/list.html")]
struct ComparisonsListTemplate {
    comparisons: Vec<utils::ComparisonMetadata>,
}

/// Comparisons list page
pub async fn comparisons_list(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let comparisons = utils::list_comparisons().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = ComparisonsListTemplate { comparisons };

    Ok(Html(
        template
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Template)]
#[template(path = "comparisons/view.html")]
struct ComparisonViewTemplate {
    from_date: String,
    to_date: String,
    records: Vec<utils::ComparisonRecord>,
    summary: Option<String>,
    charts: Vec<utils::ChartFile>,
}

/// Comparison view page
pub async fn comparison_view(
    State(_state): State<AppState>,
    Path((from_date, to_date)): Path<(String, String)>,
) -> Result<Html<String>, StatusCode> {
    // Find the comparison
    let comparisons = utils::list_comparisons().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let comparison = comparisons
        .iter()
        .find(|c| c.from_date == from_date && c.to_date == to_date)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Read data
    let records = utils::read_comparison_csv(&comparison.csv_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let summary = comparison
        .summary_path
        .as_ref()
        .and_then(|p| utils::read_summary_markdown(p).ok());

    let template = ComparisonViewTemplate {
        from_date: from_date.clone(),
        to_date: to_date.clone(),
        records,
        summary,
        charts: comparison.chart_paths.clone(),
    };

    Ok(Html(
        template
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

// ============================================================================
// Market Cap Snapshot Page Handlers
// ============================================================================

#[derive(Template)]
#[template(path = "market_caps/list.html")]
struct MarketCapsListTemplate {
    snapshots: Vec<utils::MarketCapMetadata>,
}

/// Market caps list page
pub async fn market_caps_list(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let snapshots = utils::list_market_caps().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = MarketCapsListTemplate { snapshots };

    Ok(Html(
        template
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Template)]
#[template(path = "market_caps/view.html")]
struct MarketCapViewTemplate {
    date: String,
    timestamp: String,
    records: Vec<utils::MarketCapRecord>,
    total_companies: usize,
}

/// Market cap view page
pub async fn market_cap_view(
    State(_state): State<AppState>,
    Path(date): Path<String>,
) -> Result<Html<String>, StatusCode> {
    // Find the market cap snapshot
    let snapshots = utils::list_market_caps().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let snapshot = snapshots
        .iter()
        .find(|s| s.date == date)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Read data
    let records = utils::read_marketcap_csv(&snapshot.csv_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = MarketCapViewTemplate {
        date: snapshot.date.clone(),
        timestamp: snapshot.timestamp.clone(),
        records,
        total_companies: snapshot.total_companies,
    };

    Ok(Html(
        template
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
