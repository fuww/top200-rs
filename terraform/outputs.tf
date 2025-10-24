# SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
#
# SPDX-License-Identifier: AGPL-3.0-only

output "project_id" {
  description = "The GCP project ID"
  value       = var.project_id
}

output "service_account_email" {
  description = "Email of the service account"
  value       = google_service_account.top200_rs.email
}

output "service_account_id" {
  description = "ID of the service account"
  value       = google_service_account.top200_rs.id
}

output "storage_bucket_name" {
  description = "Name of the Cloud Storage bucket for data"
  value       = google_storage_bucket.top200_data.name
}

output "storage_bucket_url" {
  description = "URL of the Cloud Storage bucket"
  value       = google_storage_bucket.top200_data.url
}

output "artifact_registry_repository" {
  description = "Artifact Registry repository name"
  value       = google_artifact_registry_repository.top200_rs.name
}

output "artifact_registry_url" {
  description = "Full URL to the Artifact Registry repository"
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.top200_rs.name}"
}

output "secrets" {
  description = "List of created secrets"
  value = {
    financialmodelingprep = google_secret_manager_secret.financialmodelingprep_api_key.secret_id
    polygon               = google_secret_manager_secret.polygon_api_key.secret_id
    anthropic             = google_secret_manager_secret.anthropic_api_key.secret_id
    brevo                 = google_secret_manager_secret.brevo_api_key.secret_id
  }
}

output "next_steps" {
  description = "Next steps after infrastructure creation"
  value = <<-EOT
    ✅ Infrastructure created successfully!

    Next steps:

    1. Set secret values:
       ./scripts/setup-secrets.sh

       Or manually:
       echo -n "YOUR_KEY" | gcloud secrets versions add financialmodelingprep-api-key --data-file=-
       echo -n "YOUR_KEY" | gcloud secrets versions add polygon-api-key --data-file=-
       echo -n "YOUR_KEY" | gcloud secrets versions add anthropic-api-key --data-file=-
       echo -n "YOUR_KEY" | gcloud secrets versions add brevo-api-key --data-file=-

    2. Configure local environment:
       export GCP_PROJECT_ID=${var.project_id}
       gcloud auth application-default login

    3. Or create a service account key for local development:
       gcloud iam service-accounts keys create key.json \\
         --iam-account=${google_service_account.top200_rs.email}
       export GOOGLE_APPLICATION_CREDENTIALS=$PWD/key.json

    4. Build and push Docker image (optional):
       gcloud builds submit --tag ${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.top200_rs.name}/top200-rs:latest

    Resources created:
    - Service Account: ${google_service_account.top200_rs.email}
    - Storage Bucket: ${google_storage_bucket.top200_data.name}
    - Artifact Registry: ${google_artifact_registry_repository.top200_rs.name}
    - Secrets: financialmodelingprep-api-key, polygon-api-key, anthropic-api-key, brevo-api-key
  EOT
}
