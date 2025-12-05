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

use crate::nats::{JobParameters, JobType};
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

/// SSE endpoint for generating comparisons (NATS-backed)
pub async fn generate_comparison_sse(
    State(state): State<AppState>,
    Query(params): Query<GenerateComparisonParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let from_date = params.from_date.clone();
    let to_date = params.to_date.clone();
    let generate_charts = params.generate_charts;
    let nats_client = state.nats_client.clone();

    let stream = async_stream::stream! {
        // Submit job to NATS
        let job_id = match crate::nats::submit_job(
            &nats_client,
            JobType::GenerateComparison,
            JobParameters::GenerateComparison {
                from_date,
                to_date,
                generate_charts,
            },
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to submit job: {}", e)));
                return;
            }
        };

        // Subscribe to job progress and result
        let progress_subject = format!("jobs.{}.progress", job_id);
        let result_subject = format!("jobs.{}.result", job_id);
        let status_subject = format!("jobs.{}.status", job_id);

        let mut progress_sub = match nats_client.inner().subscribe(progress_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to progress: {}", e)));
                return;
            }
        };

        let mut result_sub = match nats_client.inner().subscribe(result_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to result: {}", e)));
                return;
            }
        };

        let mut status_sub = match nats_client.inner().subscribe(status_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to status: {}", e)));
                return;
            }
        };

        loop {
            tokio::select! {
                Some(msg) = progress_sub.next() => {
                    if let Ok(progress) = serde_json::from_slice::<crate::nats::JobProgress>(&msg.payload) {
                        yield Ok(create_step_event(progress.step, &progress.message));
                    }
                }
                Some(msg) = status_sub.next() => {
                    if let Ok(status) = serde_json::from_slice::<crate::nats::JobStatus>(&msg.payload) {
                        if let Some(error) = status.error {
                            yield Ok(create_error_event(&error));
                            break;
                        }
                    }
                }
                Some(msg) = result_sub.next() => {
                    if let Ok(result) = serde_json::from_slice::<crate::nats::JobResult>(&msg.payload) {
                        if result.status == crate::nats::models::JobResultStatus::Success {
                            yield Ok(create_success_event());
                        } else if let Some(error) = result.error {
                            yield Ok(create_error_event(&error));
                        }
                        break;
                    }
                }
            }
        }
    };

    Sse::new(stream)
}

/// SSE endpoint for fetching market caps (NATS-backed)
pub async fn fetch_market_caps_sse(
    State(state): State<AppState>,
    Query(params): Query<FetchMarketCapsParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let date = params.date.clone();
    let nats_client = state.nats_client.clone();

    let stream = async_stream::stream! {
        // Submit job to NATS
        let job_id = match crate::nats::submit_job(
            &nats_client,
            JobType::FetchMarketCaps,
            JobParameters::FetchMarketCaps { date },
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to submit job: {}", e)));
                return;
            }
        };

        // Subscribe to job progress and result
        let progress_subject = format!("jobs.{}.progress", job_id);
        let result_subject = format!("jobs.{}.result", job_id);
        let status_subject = format!("jobs.{}.status", job_id);

        let mut progress_sub = match nats_client.inner().subscribe(progress_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to progress: {}", e)));
                return;
            }
        };

        let mut result_sub = match nats_client.inner().subscribe(result_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to result: {}", e)));
                return;
            }
        };

        let mut status_sub = match nats_client.inner().subscribe(status_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to status: {}", e)));
                return;
            }
        };

        loop {
            tokio::select! {
                Some(msg) = progress_sub.next() => {
                    if let Ok(progress) = serde_json::from_slice::<crate::nats::JobProgress>(&msg.payload) {
                        yield Ok(create_step_event(progress.step, &progress.message));
                    }
                }
                Some(msg) = status_sub.next() => {
                    if let Ok(status) = serde_json::from_slice::<crate::nats::JobStatus>(&msg.payload) {
                        if let Some(error) = status.error {
                            yield Ok(create_error_event(&error));
                            break;
                        }
                    }
                }
                Some(msg) = result_sub.next() => {
                    if let Ok(result) = serde_json::from_slice::<crate::nats::JobResult>(&msg.payload) {
                        if result.status == crate::nats::models::JobResultStatus::Success {
                            yield Ok(create_success_event());
                        } else if let Some(error) = result.error {
                            yield Ok(create_error_event(&error));
                        }
                        break;
                    }
                }
            }
        }
    };

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

/// SSE endpoint to reconnect to an existing job
pub async fn job_progress_sse(
    State(state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let nats_client = state.nats_client.clone();

    let stream = async_stream::stream! {
        // Subscribe to job progress and result
        let progress_subject = format!("jobs.{}.progress", job_id);
        let result_subject = format!("jobs.{}.result", job_id);
        let status_subject = format!("jobs.{}.status", job_id);

        let mut progress_sub = match nats_client.inner().subscribe(progress_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to progress: {}", e)));
                return;
            }
        };

        let mut result_sub = match nats_client.inner().subscribe(result_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to result: {}", e)));
                return;
            }
        };

        let mut status_sub = match nats_client.inner().subscribe(status_subject).await {
            Ok(sub) => sub,
            Err(e) => {
                yield Ok(create_error_event(&format!("Failed to subscribe to status: {}", e)));
                return;
            }
        };

        loop {
            tokio::select! {
                Some(msg) = progress_sub.next() => {
                    if let Ok(progress) = serde_json::from_slice::<crate::nats::JobProgress>(&msg.payload) {
                        yield Ok(create_step_event(progress.step, &progress.message));
                    }
                }
                Some(msg) = status_sub.next() => {
                    if let Ok(status) = serde_json::from_slice::<crate::nats::JobStatus>(&msg.payload) {
                        if let Some(error) = status.error {
                            yield Ok(create_error_event(&error));
                            break;
                        }
                    }
                }
                Some(msg) = result_sub.next() => {
                    if let Ok(result) = serde_json::from_slice::<crate::nats::JobResult>(&msg.payload) {
                        if result.status == crate::nats::models::JobResultStatus::Success {
                            yield Ok(create_success_event());
                        } else if let Some(error) = result.error {
                            yield Ok(create_error_event(&error));
                        }
                        break;
                    }
                }
            }
        }
    };

    Sse::new(stream)
}
