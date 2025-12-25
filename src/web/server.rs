// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{Json, Router, routing::get};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

use crate::web::{routes, state::AppState};

/// Create the Axum router with all routes
pub fn create_app(state: AppState) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // Authentication routes (no auth required)
        .route("/login", get(routes::auth::login_page))
        .route("/api/auth/callback", get(routes::auth::auth_callback))
        .route("/api/auth/logout", get(routes::auth::logout))
        // Dashboard page (will require auth later)
        .route("/", get(routes::pages::dashboard))
        // Comparison pages
        .route("/comparisons", get(routes::pages::comparisons_list))
        .route("/comparisons/new", get(routes::pages::new_comparison))
        .route(
            "/comparisons/:from/:to",
            get(routes::pages::comparison_view),
        )
        // Market cap pages
        .route("/market-caps", get(routes::pages::market_caps_list))
        .route(
            "/market-caps/fetch",
            get(routes::pages::fetch_market_caps_page),
        )
        .route("/market-caps/:date", get(routes::pages::market_cap_view))
        // API endpoints
        .route("/api/comparisons", get(routes::api::list_comparisons))
        .route(
            "/api/comparisons/:from/:to",
            get(routes::api::get_comparison),
        )
        .route("/api/charts/:from/:to/:type", get(routes::api::get_chart))
        .route("/api/market-caps", get(routes::api::list_market_caps))
        .route("/api/market-caps/:date", get(routes::api::get_market_cap))
        // Job management endpoints
        .route("/api/jobs/:job_id", get(routes::api::get_job_status))
        // SSE endpoints for data generation
        .route(
            "/api/generate-comparison-sse",
            get(routes::sse::generate_comparison_sse),
        )
        .route(
            "/api/fetch-market-caps-sse",
            get(routes::sse::fetch_market_caps_sse),
        )
        .route(
            "/api/jobs/:job_id/progress",
            get(routes::sse::job_progress_sse),
        )
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        // Share app state
        .with_state(state)
}

/// Start the web server
pub async fn start_server(state: AppState, port: u16) -> anyhow::Result<()> {
    let app = create_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🚀 Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
