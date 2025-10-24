// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use google_secretmanager1::api::SecretManagerHub;
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use std::env;
use yup_oauth2::ServiceAccountAuthenticator;

/// Configuration for accessing Google Secret Manager
#[derive(Debug, Clone)]
pub struct SecretManagerConfig {
    /// GCP Project ID
    pub project_id: String,
    /// Whether to fall back to environment variables if Secret Manager fails
    pub fallback_to_env: bool,
}

impl Default for SecretManagerConfig {
    fn default() -> Self {
        Self {
            project_id: env::var("GCP_PROJECT_ID").unwrap_or_else(|_| String::new()),
            fallback_to_env: true,
        }
    }
}

/// Client for accessing secrets from Google Secret Manager
pub struct SecretManager {
    hub: SecretManagerHub<HttpsConnector<HttpConnector>>,
    config: SecretManagerConfig,
}

impl SecretManager {
    /// Create a new SecretManager client
    ///
    /// This will attempt to authenticate using:
    /// 1. GOOGLE_APPLICATION_CREDENTIALS environment variable (service account key file)
    /// 2. Application Default Credentials (ADC) - used in GCP environments
    pub async fn new(config: SecretManagerConfig) -> Result<Self> {
        let auth = if let Ok(key_file) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            // Use service account key file
            let service_account_key = yup_oauth2::read_service_account_key(&key_file)
                .await
                .context("Failed to read service account key")?;

            ServiceAccountAuthenticator::builder(service_account_key)
                .build()
                .await
                .context("Failed to create authenticator from service account")?
        } else {
            // Use Application Default Credentials
            ServiceAccountAuthenticator::builder(yup_oauth2::ServiceAccountKey {
                key_type: Some("service_account".to_string()),
                project_id: Some(config.project_id.clone()),
                private_key_id: None,
                private_key: String::new(),
                client_email: String::new(),
                client_id: None,
                auth_uri: None,
                token_uri: None,
                auth_provider_x509_cert_url: None,
                client_x509_cert_url: None,
            })
            .build()
            .await
            .context("Failed to create authenticator with ADC")?
        };

        let client = hyper::Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_webpki_roots()
                .https_or_http()
                .enable_http1()
                .build(),
        );

        let hub = SecretManagerHub::new(client, auth);

        Ok(Self { hub, config })
    }

    /// Get the latest version of a secret
    ///
    /// # Arguments
    /// * `secret_name` - The name of the secret (e.g., "financialmodelingprep-api-key")
    ///
    /// # Returns
    /// The secret value as a String
    pub async fn get_secret(&self, secret_name: &str) -> Result<String> {
        let secret_path = format!(
            "projects/{}/secrets/{}/versions/latest",
            self.config.project_id, secret_name
        );

        let result = self
            .hub
            .projects()
            .secrets_versions_access(&secret_path)
            .doit()
            .await;

        match result {
            Ok((_response, secret_version)) => {
                let payload = secret_version.payload.context("Secret has no payload")?;
                let data = payload.data.context("Secret payload has no data")?;
                let decoded = String::from_utf8(data).context("Secret data is not valid UTF-8")?;
                Ok(decoded)
            }
            Err(e) if self.config.fallback_to_env => {
                // Try to fall back to environment variable
                eprintln!(
                    "Warning: Failed to fetch secret '{}' from Secret Manager: {}",
                    secret_name, e
                );
                eprintln!("Falling back to environment variable");

                // Convert secret name to environment variable format
                let env_var_name = secret_name.to_uppercase().replace('-', "_");
                env::var(&env_var_name).with_context(|| {
                    format!(
                        "Failed to get secret from Secret Manager or environment variable {}",
                        env_var_name
                    )
                })
            }
            Err(e) => Err(e).context(format!("Failed to fetch secret '{}'", secret_name)),
        }
    }

    /// Get multiple secrets at once
    ///
    /// # Arguments
    /// * `secret_names` - A slice of secret names to fetch
    ///
    /// # Returns
    /// A vector of tuples containing (secret_name, secret_value)
    pub async fn get_secrets(&self, secret_names: &[&str]) -> Result<Vec<(String, String)>> {
        let mut results = Vec::new();

        for name in secret_names {
            let value = self.get_secret(name).await?;
            results.push((name.to_string(), value));
        }

        Ok(results)
    }
}

/// Get a secret value, trying Secret Manager first, then falling back to environment variable
///
/// This is a convenience function for simple use cases.
///
/// # Arguments
/// * `secret_name` - The name of the secret in Secret Manager (e.g., "financialmodelingprep-api-key")
/// * `env_var_name` - The environment variable name to use as fallback (e.g., "FINANCIALMODELINGPREP_API_KEY")
///
/// # Returns
/// The secret value as a String
pub async fn get_secret_or_env(secret_name: &str, env_var_name: &str) -> Result<String> {
    // First try environment variable (for local development)
    if let Ok(value) = env::var(env_var_name) {
        return Ok(value);
    }

    // Then try Secret Manager (for production)
    if let Ok(project_id) = env::var("GCP_PROJECT_ID") {
        let config = SecretManagerConfig {
            project_id,
            fallback_to_env: false,
        };

        match SecretManager::new(config).await {
            Ok(sm) => return sm.get_secret(secret_name).await,
            Err(e) => {
                eprintln!("Warning: Failed to initialize Secret Manager: {}", e);
            }
        }
    }

    Err(anyhow::anyhow!(
        "Secret '{}' not found in environment variable '{}' or Secret Manager",
        secret_name,
        env_var_name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SecretManagerConfig::default();
        assert!(config.fallback_to_env);
    }
}
