// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod client;
pub mod jobs;
pub mod models;
pub mod streams;
pub mod worker;

pub use client::{NatsClient, create_nats_client};
pub use jobs::{publish_job_progress, publish_job_result, publish_job_status, submit_job};
pub use models::{JobParameters, JobProgress, JobRequest, JobResult, JobStatus, JobType};
pub use streams::setup_streams;
pub use worker::start_worker;
