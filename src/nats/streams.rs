// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use async_nats::jetstream::stream::{Config, DiscardPolicy, RetentionPolicy};
use std::time::Duration;

use super::NatsClient;

const JOBS_SUBMIT_STREAM: &str = "JOBS_SUBMIT";
const JOBS_TRACKING_STREAM: &str = "JOBS_TRACKING";

/// Set up JetStream streams for job submission and tracking
pub async fn setup_streams(nats_client: &NatsClient) -> Result<()> {
    let jetstream = async_nats::jetstream::new(nats_client.inner().clone());

    // Create JOBS_SUBMIT stream (WorkQueue retention)
    let submit_config = Config {
        name: JOBS_SUBMIT_STREAM.to_string(),
        description: Some("Job submission queue".to_string()),
        subjects: vec!["jobs.submit.>".to_string()],
        retention: RetentionPolicy::WorkQueue,
        max_age: Duration::from_secs(24 * 60 * 60), // 24 hours
        max_messages: 10000,
        discard: DiscardPolicy::Old,
        ..Default::default()
    };

    match jetstream.get_or_create_stream(submit_config).await {
        Ok(_) => println!("✓ JetStream stream '{}' ready", JOBS_SUBMIT_STREAM),
        Err(e) => {
            eprintln!(
                "Warning: Failed to create stream {}: {}",
                JOBS_SUBMIT_STREAM, e
            );
        }
    }

    // Create JOBS_TRACKING stream (Limits retention)
    let tracking_config = Config {
        name: JOBS_TRACKING_STREAM.to_string(),
        description: Some("Job status and progress tracking".to_string()),
        subjects: vec![
            "jobs.*.status".to_string(),
            "jobs.*.progress".to_string(),
            "jobs.*.result".to_string(),
        ],
        retention: RetentionPolicy::Limits,
        max_age: Duration::from_secs(60 * 60), // 1 hour
        max_messages_per_subject: 100,
        discard: DiscardPolicy::Old,
        ..Default::default()
    };

    match jetstream.get_or_create_stream(tracking_config).await {
        Ok(_) => println!("✓ JetStream stream '{}' ready", JOBS_TRACKING_STREAM),
        Err(e) => {
            eprintln!(
                "Warning: Failed to create stream {}: {}",
                JOBS_TRACKING_STREAM, e
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nats::create_nats_client;

    #[tokio::test]
    #[ignore] // Requires NATS server running
    async fn test_setup_streams() {
        let client = create_nats_client("nats://127.0.0.1:4222").await.unwrap();
        let result = setup_streams(&client).await;
        assert!(result.is_ok());
    }
}
