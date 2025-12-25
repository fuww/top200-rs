// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::process::Command;

use super::{
    JobParameters, JobProgress, JobRequest, JobResult, JobStatus, JobType, NatsClient,
    publish_job_progress, publish_job_result, publish_job_status,
};

/// Start the background worker that processes jobs from NATS queue
pub async fn start_worker(nats_client: NatsClient) -> Result<()> {
    println!("🚀 Starting NATS worker...");

    // Subscribe to job submissions
    let mut sub = nats_client
        .inner()
        .subscribe("jobs.submit.>".to_string())
        .await
        .context("Failed to subscribe to job queue")?;

    println!("✓ Worker subscribed to jobs.submit.>");

    // Process messages in a loop
    while let Some(msg) = sub.next().await {
        // Deserialize job request
        let job_request: JobRequest = match serde_json::from_slice(&msg.payload) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Failed to deserialize job request: {}", e);
                continue;
            }
        };

        println!(
            "📋 Received job: {} ({})",
            job_request.job_id,
            match &job_request.job_type {
                JobType::FetchMarketCaps => "fetch-market-caps",
                JobType::GenerateComparison => "comparison",
            }
        );

        // Clone for async task
        let client = nats_client.clone();
        let job_id = job_request.job_id.clone();

        // Spawn task to process job
        tokio::spawn(async move {
            if let Err(e) = process_job(&client, job_request).await {
                eprintln!("❌ Job {} failed: {}", job_id, e);

                // Publish failure status and result
                let _ = publish_job_status(
                    &client,
                    JobStatus::new_failed(job_id.clone(), e.to_string()),
                )
                .await;
                let _ = publish_job_result(&client, JobResult::failed(job_id, e.to_string())).await;
            }
        });
    }

    Ok(())
}

/// Process a single job
async fn process_job(nats_client: &NatsClient, job_request: JobRequest) -> Result<()> {
    let job_id = job_request.job_id.clone();

    match job_request.job_type {
        JobType::FetchMarketCaps => {
            execute_fetch_market_caps(nats_client, job_id, job_request.parameters).await
        }
        JobType::GenerateComparison => {
            execute_generate_comparison(nats_client, job_id, job_request.parameters).await
        }
    }
}

/// Execute fetch market caps job
async fn execute_fetch_market_caps(
    nats_client: &NatsClient,
    job_id: String,
    parameters: JobParameters,
) -> Result<()> {
    let date = match parameters {
        JobParameters::FetchMarketCaps { date } => date,
        _ => anyhow::bail!("Invalid parameters for FetchMarketCaps job"),
    };

    // Update status to running
    publish_job_status(
        nats_client,
        JobStatus::new_running(
            job_id.clone(),
            1,
            format!("Fetching market caps for {}", date),
        ),
    )
    .await?;

    publish_job_progress(
        nats_client,
        JobProgress::new(
            job_id.clone(),
            1,
            format!("Starting market cap fetch for {}", date),
            None,
        ),
    )
    .await?;

    // Execute cargo command
    let output = Command::new("cargo")
        .args(&["run", "--", "fetch-specific-date-market-caps", &date])
        .envs(std::env::vars())
        .output()
        .await
        .context("Failed to execute cargo command")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("Command failed: {}", error_msg);
    }

    // Parse output to find generated files
    let stdout = String::from_utf8_lossy(&output.stdout);
    let output_files = extract_output_files(&stdout);

    // Publish success
    publish_job_status(nats_client, JobStatus::new_completed(job_id.clone())).await?;
    publish_job_result(nats_client, JobResult::success(job_id, output_files)).await?;

    Ok(())
}

/// Execute generate comparison job
async fn execute_generate_comparison(
    nats_client: &NatsClient,
    job_id: String,
    parameters: JobParameters,
) -> Result<()> {
    let (from_date, to_date, generate_charts) = match parameters {
        JobParameters::GenerateComparison {
            from_date,
            to_date,
            generate_charts,
        } => (from_date, to_date, generate_charts),
        _ => anyhow::bail!("Invalid parameters for GenerateComparison job"),
    };

    // Step 1: Fetch market caps for from_date
    publish_job_status(
        nats_client,
        JobStatus::new_running(
            job_id.clone(),
            1,
            "Fetching from date market caps...".to_string(),
        ),
    )
    .await?;

    publish_job_progress(
        nats_client,
        JobProgress::new(
            job_id.clone(),
            1,
            format!("Fetching market caps for {}", from_date),
            None,
        ),
    )
    .await?;

    let output = Command::new("cargo")
        .args(&["run", "--", "fetch-specific-date-market-caps", &from_date])
        .envs(std::env::vars())
        .output()
        .await
        .context("Failed to fetch from date market caps")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("Failed to fetch from date: {}", error_msg);
    }

    // Step 2: Fetch market caps for to_date
    publish_job_status(
        nats_client,
        JobStatus::new_running(
            job_id.clone(),
            2,
            "Fetching to date market caps...".to_string(),
        ),
    )
    .await?;

    publish_job_progress(
        nats_client,
        JobProgress::new(
            job_id.clone(),
            2,
            format!("Fetching market caps for {}", to_date),
            None,
        ),
    )
    .await?;

    let output = Command::new("cargo")
        .args(&["run", "--", "fetch-specific-date-market-caps", &to_date])
        .envs(std::env::vars())
        .output()
        .await
        .context("Failed to fetch to date market caps")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("Failed to fetch to date: {}", error_msg);
    }

    // Step 3: Generate comparison
    publish_job_status(
        nats_client,
        JobStatus::new_running(job_id.clone(), 3, "Generating comparison...".to_string()),
    )
    .await?;

    publish_job_progress(
        nats_client,
        JobProgress::new(
            job_id.clone(),
            3,
            "Generating comparison report".to_string(),
            None,
        ),
    )
    .await?;

    let output = Command::new("cargo")
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
        .await
        .context("Failed to generate comparison")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("Failed to generate comparison: {}", error_msg);
    }

    let mut output_files = extract_output_files(&String::from_utf8_lossy(&output.stdout));

    // Step 4: Generate charts (if requested)
    if generate_charts {
        publish_job_status(
            nats_client,
            JobStatus::new_running(job_id.clone(), 4, "Generating charts...".to_string()),
        )
        .await?;

        publish_job_progress(
            nats_client,
            JobProgress::new(
                job_id.clone(),
                4,
                "Generating visualization charts".to_string(),
                None,
            ),
        )
        .await?;

        let output = Command::new("cargo")
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
            .await
            .context("Failed to generate charts")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("Failed to generate charts: {}", error_msg);
        }

        let chart_files = extract_output_files(&String::from_utf8_lossy(&output.stdout));
        output_files.extend(chart_files);
    }

    // Publish success
    publish_job_status(nats_client, JobStatus::new_completed(job_id.clone())).await?;
    publish_job_result(nats_client, JobResult::success(job_id, output_files)).await?;

    Ok(())
}

/// Extract output file paths from command stdout
fn extract_output_files(stdout: &str) -> Vec<String> {
    let mut files = Vec::new();

    // Look for patterns like "output/..." or "Generated: ..."
    for line in stdout.lines() {
        if line.contains("output/") {
            // Extract file path
            if let Some(start) = line.find("output/") {
                let rest = &line[start..];
                if let Some(end) = rest.find(|c: char| c.is_whitespace() || c == ',' || c == ')') {
                    files.push(rest[..end].to_string());
                } else {
                    files.push(rest.to_string());
                }
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_output_files() {
        let stdout = "Generated comparison at output/comparison_2025-01-01_to_2025-02-01.csv\n\
                      Summary written to output/comparison_2025-01-01_to_2025-02-01_summary.md";

        let files = extract_output_files(stdout);
        assert_eq!(files.len(), 2);
        assert!(files[0].starts_with("output/comparison"));
    }
}
