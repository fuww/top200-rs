// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::Utc;
use uuid::Uuid;

use super::{JobParameters, JobProgress, JobRequest, JobResult, JobStatus, JobType, NatsClient};

/// Submit a new job to NATS queue
pub async fn submit_job(
    nats_client: &NatsClient,
    job_type: JobType,
    parameters: JobParameters,
) -> Result<String> {
    let job_id = Uuid::new_v4().to_string();

    let job_request = JobRequest {
        job_id: job_id.clone(),
        job_type: job_type.clone(),
        parameters,
        submitted_at: Utc::now(),
    };

    let subject = match job_type {
        JobType::FetchMarketCaps => "jobs.submit.fetch-market-caps",
        JobType::GenerateComparison => "jobs.submit.comparison",
    };

    let payload = serde_json::to_vec(&job_request)
        .context("Failed to serialize job request")?;

    nats_client
        .inner()
        .publish(subject.to_string(), payload.into())
        .await
        .context("Failed to publish job to NATS")?;

    // Publish initial status
    publish_job_status(nats_client, JobStatus::new_queued(job_id.clone())).await?;

    Ok(job_id)
}

/// Publish job status update
pub async fn publish_job_status(nats_client: &NatsClient, status: JobStatus) -> Result<()> {
    let subject = format!("jobs.{}.status", status.job_id);
    let payload = serde_json::to_vec(&status)
        .context("Failed to serialize job status")?;

    nats_client
        .inner()
        .publish(subject, payload.into())
        .await
        .context("Failed to publish job status")?;

    Ok(())
}

/// Publish job progress update
pub async fn publish_job_progress(nats_client: &NatsClient, progress: JobProgress) -> Result<()> {
    let subject = format!("jobs.{}.progress", progress.job_id);
    let payload = serde_json::to_vec(&progress)
        .context("Failed to serialize job progress")?;

    nats_client
        .inner()
        .publish(subject, payload.into())
        .await
        .context("Failed to publish job progress")?;

    Ok(())
}

/// Publish job result (final outcome)
pub async fn publish_job_result(nats_client: &NatsClient, result: JobResult) -> Result<()> {
    let subject = format!("jobs.{}.result", result.job_id);
    let payload = serde_json::to_vec(&result)
        .context("Failed to serialize job result")?;

    nats_client
        .inner()
        .publish(subject, payload.into())
        .await
        .context("Failed to publish job result")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nats::create_nats_client;

    #[tokio::test]
    #[ignore] // Requires NATS server running
    async fn test_submit_job() {
        let client = create_nats_client("nats://127.0.0.1:4222").await.unwrap();

        let job_id = submit_job(
            &client,
            JobType::FetchMarketCaps,
            JobParameters::FetchMarketCaps {
                date: "2025-01-01".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(!job_id.is_empty());
        assert!(Uuid::parse_str(&job_id).is_ok());
    }
}
