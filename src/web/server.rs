// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    routing::get,
    Router,
    Json,
};
use serde_json::json;
use tower_http::services::ServeDir;
use std::net::SocketAddr;

use crate::web::{routes, state::AppState};

/// Create the Axum router with all routes
pub fn create_app(state: AppState) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // Dashboard page
        .route("/", get(routes::pages::dashboard))
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
