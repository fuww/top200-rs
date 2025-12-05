// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use futures::{
    stream::{self, Stream},
    StreamExt,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

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

    let stream = stream::iter(
        async move {
            let mut events = Vec::new();

            // Step 1: Fetch market caps for from_date
            events.push(create_step_event(
                1,
                "Fetching market caps for from date...",
            ));

            let result = Command::new("cargo")
                .args(&["run", "--", "fetch-specific-date-market-caps", &from_date])
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    events.push(create_step_event(1, "✓ From date market caps fetched"));
                }
                Ok(output) => {
                    let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                    events.push(create_error_event(&format!(
                        "Failed to fetch from date market caps: {}",
                        error_msg
                    )));
                    return events;
                }
                Err(e) => {
                    events.push(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )));
                    return events;
                }
            }

            sleep(Duration::from_millis(500)).await;

            // Step 2: Fetch market caps for to_date
            events.push(create_step_event(2, "Fetching market caps for to date..."));

            let result = Command::new("cargo")
                .args(&["run", "--", "fetch-specific-date-market-caps", &to_date])
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    events.push(create_step_event(2, "✓ To date market caps fetched"));
                }
                Ok(output) => {
                    let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                    events.push(create_error_event(&format!(
                        "Failed to fetch to date market caps: {}",
                        error_msg
                    )));
                    return events;
                }
                Err(e) => {
                    events.push(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )));
                    return events;
                }
            }

            sleep(Duration::from_millis(500)).await;

            // Step 3: Generate comparison
            events.push(create_step_event(3, "Generating comparison report..."));

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
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    events.push(create_step_event(3, "✓ Comparison report generated"));
                }
                Ok(output) => {
                    let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                    events.push(create_error_event(&format!(
                        "Failed to generate comparison: {}",
                        error_msg
                    )));
                    return events;
                }
                Err(e) => {
                    events.push(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )));
                    return events;
                }
            }

            sleep(Duration::from_millis(500)).await;

            // Step 4: Generate charts (if requested)
            if generate_charts {
                events.push(create_step_event(4, "Generating visualization charts..."));

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
                    .output()
                    .await;

                match result {
                    Ok(output) if output.status.success() => {
                        events.push(create_step_event(4, "✓ Charts generated"));
                    }
                    Ok(output) => {
                        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                        events.push(create_error_event(&format!(
                            "Failed to generate charts: {}",
                            error_msg
                        )));
                        return events;
                    }
                    Err(e) => {
                        events.push(create_error_event(&format!(
                            "Failed to execute command: {}",
                            e
                        )));
                        return events;
                    }
                }
            }

            // Success!
            events.push(create_success_event());

            events
        }
        .await,
    );

    Sse::new(stream.map(|event| Ok(event)))
}

/// SSE endpoint for fetching market caps
pub async fn fetch_market_caps_sse(
    State(state): State<AppState>,
    Query(params): Query<FetchMarketCapsParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let date = params.date.clone();
    let config = state.config.clone();

    let stream = stream::iter(
        async move {
            let mut events = Vec::new();

            // Get total number of tickers
            let total_tickers = config.us_tickers.len() + config.non_us_tickers.len();

            events.push(create_progress_event(0, total_tickers, "Starting..."));

            // Execute the fetch command
            let result = Command::new("cargo")
                .args(&["run", "--", "fetch-specific-date-market-caps", &date])
                .output()
                .await;

            match result {
                Ok(output) if output.status.success() => {
                    // Simulate progress updates (in real implementation, you'd parse output)
                    for i in 1..=total_tickers {
                        let ticker = if i <= config.us_tickers.len() {
                            config.us_tickers.get(i - 1).cloned()
                        } else {
                            config
                                .non_us_tickers
                                .get(i - config.us_tickers.len() - 1)
                                .cloned()
                        };

                        events.push(create_progress_event(
                            i,
                            total_tickers,
                            ticker.as_deref().unwrap_or("Unknown"),
                        ));

                        sleep(Duration::from_millis(100)).await;
                    }

                    events.push(create_success_event());
                }
                Ok(output) => {
                    let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                    events.push(create_error_event(&format!(
                        "Failed to fetch market caps: {}",
                        error_msg
                    )));
                }
                Err(e) => {
                    events.push(create_error_event(&format!(
                        "Failed to execute command: {}",
                        e
                    )));
                }
            }

            events
        }
        .await,
    );

    Sse::new(stream.map(|event| Ok(event)))
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
