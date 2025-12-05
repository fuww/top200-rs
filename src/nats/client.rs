// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use async_nats::{Client, ConnectOptions};

/// NATS client wrapper
#[derive(Clone)]
pub struct NatsClient {
    client: Client,
}

impl NatsClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }
}

/// Create and connect to NATS server
pub async fn create_nats_client(nats_url: &str) -> Result<NatsClient> {
    let client = ConnectOptions::new()
        .name("top200-rs")
        .connect(nats_url)
        .await
        .with_context(|| format!("Failed to connect to NATS server at {}", nats_url))?;

    println!("✓ Connected to NATS server at {}", nats_url);

    Ok(NatsClient::new(client))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires NATS server running
    async fn test_connect_to_nats() {
        let result = create_nats_client("nats://127.0.0.1:4222").await;
        assert!(result.is_ok());
    }
}
