// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;

use crate::web::state::AppState;

#[derive(Debug, Deserialize)]
pub struct GenerateComparisonParams {
    pub from_date: String,
    pub to_date: String,
    #[serde(default)]
    pub generate_charts: bool,
}

#[derive(Debug, Deserialize)]
pub struct FetchMarketCapsParams {
    pub date: String,
}

#[derive(Debug, Serialize)]
struct SseMessage {
    #[serde(rename = "type")]
    msg_type: String,
    step: Option<u8>,
    message: Option<String>,
    progress: Option<Progress>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Progress {
    current: usize,
    total: usize,
    ticker: Option<String>,
}

/// SSE endpoint for generating comparisons
pub async fn generate_comparison_sse(
    State(_state): State<AppState>,
    Query(params): Query<GenerateComparisonParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let from_date = params.from_date.clone();
    let to_date = params.to_date.clone();
    let generate_charts = params.generate_charts;

    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        // Step 1: Fetch market caps for from_date
        let _ = tx
            .send(create_step_event(
                1,
                "Fetching market caps for from date...",
            ))
            .await;

        let result = Command::new("cargo")
            .args(&["run", "--", "fetch-specific-date-market-caps", &from_date])
            .envs(std::env::vars())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let _ = tx
                    .send(create_step_event(1, "✓ From date market caps fetched"))
                    .await;
            }
            Ok(output) => {
                let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to fetch from date market caps: {}",
                        error_msg
                    )))
                    .await;
                return;
            }
            Err(e) => {
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )))
                    .await;
                return;
            }
        }

        sleep(Duration::from_millis(500)).await;

        // Step 2: Fetch market caps for to_date
        let _ = tx
            .send(create_step_event(2, "Fetching market caps for to date..."))
            .await;

        let result = Command::new("cargo")
            .args(&["run", "--", "fetch-specific-date-market-caps", &to_date])
            .envs(std::env::vars())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let _ = tx
                    .send(create_step_event(2, "✓ To date market caps fetched"))
                    .await;
            }
            Ok(output) => {
                let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to fetch to date market caps: {}",
                        error_msg
                    )))
                    .await;
                return;
            }
            Err(e) => {
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )))
                    .await;
                return;
            }
        }

        sleep(Duration::from_millis(500)).await;

        // Step 3: Generate comparison
        let _ = tx
            .send(create_step_event(3, "Generating comparison report..."))
            .await;

        let result = Command::new("cargo")
            .args(&[
                "run",
                "--",
                "compare-market-caps",
                "--from",
                &from_date,
                "--to",
                &to_date,
            ])
            .envs(std::env::vars())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let _ = tx
                    .send(create_step_event(3, "✓ Comparison report generated"))
                    .await;
            }
            Ok(output) => {
                let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to generate comparison: {}",
                        error_msg
                    )))
                    .await;
                return;
            }
            Err(e) => {
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )))
                    .await;
                return;
            }
        }

        sleep(Duration::from_millis(500)).await;

        // Step 4: Generate charts (if requested)
        if generate_charts {
            let _ = tx
                .send(create_step_event(4, "Generating visualization charts..."))
                .await;

            let result = Command::new("cargo")
                .args(&[
                    "run",
                    "--",
                    "generate-charts",
                    "--from",
                    &from_date,
                    "--to",
                    &to_date,
                ])
                .envs(std::env::vars())
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    let _ = tx.send(create_step_event(4, "✓ Charts generated")).await;
                }
                Ok(output) => {
                    let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                    let _ = tx
                        .send(create_error_event(&format!(
                            "Failed to generate charts: {}",
                            error_msg
                        )))
                        .await;
                    return;
                }
                Err(e) => {
                    let _ = tx
                        .send(create_error_event(&format!(
                            "Failed to execute command: {}",
                            e
                        )))
                        .await;
                    return;
                }
            }
        }

        // Success!
        let _ = tx.send(create_success_event()).await;
    });

    let stream = ReceiverStream::new(rx).map(Ok);
    Sse::new(stream)
}

/// SSE endpoint for fetching market caps
pub async fn fetch_market_caps_sse(
    State(state): State<AppState>,
    Query(params): Query<FetchMarketCapsParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let date = params.date.clone();
    let config = state.config.clone();

    let (tx, rx) = mpsc::channel(32);

    tokio::spawn(async move {
        // Get total number of tickers
        let total_tickers = config.us_tickers.len() + config.non_us_tickers.len();

        let _ = tx
            .send(create_step_event(
                1,
                &format!("Fetching market caps for {} tickers...", total_tickers),
            ))
            .await;

        // Execute the fetch command
        let result = Command::new("cargo")
            .args(&["run", "--", "fetch-specific-date-market-caps", &date])
            .envs(std::env::vars())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let _ = tx.send(create_success_event()).await;
            }
            Ok(output) => {
                let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to fetch market caps: {}",
                        error_msg
                    )))
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )))
                    .await;
            }
        }
    });

    let stream = ReceiverStream::new(rx).map(Ok);
    Sse::new(stream)
}

// Helper functions to create SSE events

fn create_step_event(step: u8, message: &str) -> Event {
    let msg = SseMessage {
        msg_type: "step".to_string(),
        step: Some(step),
        message: Some(message.to_string()),
        progress: None,
        error: None,
    };

    Event::default().json_data(msg).unwrap()
}

fn create_progress_event(current: usize, total: usize, ticker: &str) -> Event {
    let msg = SseMessage {
        msg_type: "progress".to_string(),
        step: None,
        message: None,
        progress: Some(Progress {
            current,
            total,
            ticker: Some(ticker.to_string()),
        }),
        error: None,
    };

    Event::default().json_data(msg).unwrap()
}

fn create_success_event() -> Event {
    let msg = SseMessage {
        msg_type: "success".to_string(),
        step: None,
        message: Some("Completed successfully!".to_string()),
        progress: None,
        error: None,
    };

    Event::default().json_data(msg).unwrap()
}

fn create_error_event(error: &str) -> Event {
    let msg = SseMessage {
        msg_type: "error".to_string(),
        step: None,
        message: None,
        progress: None,
        error: Some(error.to_string()),
    };

    Event::default().json_data(msg).unwrap()
}
