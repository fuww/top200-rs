// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Job request that gets published to NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub job_id: String,
    pub job_type: JobType,
    pub parameters: JobParameters,
    pub submitted_at: DateTime<Utc>,
}

/// Types of jobs that can be submitted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    FetchMarketCaps,
    GenerateComparison,
}

/// Parameters for different job types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobParameters {
    FetchMarketCaps { date: String },
    GenerateComparison {
        from_date: String,
        to_date: String,
        generate_charts: bool,
    },
}

/// Job status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    pub job_id: String,
    pub status: JobStatusType,
    pub current_step: Option<u8>,
    pub current_step_message: Option<String>,
    pub error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Possible job statuses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatusType {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Progress updates during job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub step: u8,
    pub message: String,
    pub ticker: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Final job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub status: JobResultStatus,
    pub output_files: Vec<String>,
    pub error: Option<String>,
    pub completed_at: DateTime<Utc>,
}

/// Result status (success or failure)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobResultStatus {
    Success,
    Failed,
}

impl JobStatus {
    pub fn new_queued(job_id: String) -> Self {
        Self {
            job_id,
            status: JobStatusType::Queued,
            current_step: None,
            current_step_message: None,
            error: None,
            updated_at: Utc::now(),
        }
    }

    pub fn new_running(job_id: String, step: u8, message: String) -> Self {
        Self {
            job_id,
            status: JobStatusType::Running,
            current_step: Some(step),
            current_step_message: Some(message),
            error: None,
            updated_at: Utc::now(),
        }
    }

    pub fn new_completed(job_id: String) -> Self {
        Self {
            job_id,
            status: JobStatusType::Completed,
            current_step: None,
            current_step_message: None,
            error: None,
            updated_at: Utc::now(),
        }
    }

    pub fn new_failed(job_id: String, error: String) -> Self {
        Self {
            job_id,
            status: JobStatusType::Failed,
            current_step: None,
            current_step_message: None,
            error: Some(error),
            updated_at: Utc::now(),
        }
    }
}

impl JobProgress {
    pub fn new(job_id: String, step: u8, message: String, ticker: Option<String>) -> Self {
        Self {
            job_id,
            step,
            message,
            ticker,
            timestamp: Utc::now(),
        }
    }
}

impl JobResult {
    pub fn success(job_id: String, output_files: Vec<String>) -> Self {
        Self {
            job_id,
            status: JobResultStatus::Success,
            output_files,
            error: None,
            completed_at: Utc::now(),
        }
    }

    pub fn failed(job_id: String, error: String) -> Self {
        Self {
            job_id,
            status: JobResultStatus::Failed,
            output_files: Vec::new(),
            error: Some(error),
            completed_at: Utc::now(),
        }
    }
}
