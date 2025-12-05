// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod client;
pub mod models;
pub mod streams;
pub mod jobs;
pub mod worker;

pub use client::{create_nats_client, NatsClient};
pub use models::{JobRequest, JobStatus, JobProgress, JobResult, JobType, JobParameters};
pub use streams::setup_streams;
pub use jobs::{publish_job_progress, publish_job_result, publish_job_status, submit_job};
pub use worker::start_worker;
