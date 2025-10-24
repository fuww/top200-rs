# SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
#
# SPDX-License-Identifier: AGPL-3.0-only

terraform {
  required_version = ">= 1.0"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }

  # Backend configuration for storing Terraform state
  # Uncomment and configure after initial setup
  # backend "gcs" {
  #   bucket = "your-terraform-state-bucket"
  #   prefix = "terraform/top200-rs"
  # }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# Enable required APIs
resource "google_project_service" "secret_manager" {
  project = var.project_id
  service = "secretmanager.googleapis.com"

  disable_dependent_services = false
  disable_on_destroy         = false
}

resource "google_project_service" "cloud_run" {
  project = var.project_id
  service = "run.googleapis.com"

  disable_dependent_services = false
  disable_on_destroy         = false
}

resource "google_project_service" "compute" {
  project = var.project_id
  service = "compute.googleapis.com"

  disable_dependent_services = false
  disable_on_destroy         = false
}

resource "google_project_service" "cloud_build" {
  project = var.project_id
  service = "cloudbuild.googleapis.com"

  disable_dependent_services = false
  disable_on_destroy         = false
}

resource "google_project_service" "artifact_registry" {
  project = var.project_id
  service = "artifactregistry.googleapis.com"

  disable_dependent_services = false
  disable_on_destroy         = false
}

resource "google_project_service" "iam" {
  project = var.project_id
  service = "iam.googleapis.com"

  disable_dependent_services = false
  disable_on_destroy         = false
}

# Create service account for the application
resource "google_service_account" "top200_rs" {
  account_id   = "top200-rs"
  display_name = "Top200-RS Service Account"
  description  = "Service account for top200-rs application"
  project      = var.project_id

  depends_on = [google_project_service.iam]
}

# Grant Secret Manager access to the service account
resource "google_project_iam_member" "secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.top200_rs.email}"

  depends_on = [google_service_account.top200_rs]
}

# Create secrets in Secret Manager
resource "google_secret_manager_secret" "financialmodelingprep_api_key" {
  secret_id = "financialmodelingprep-api-key"
  project   = var.project_id

  labels = {
    app        = "top200-rs"
    managed-by = "terraform"
  }

  replication {
    auto {}
  }

  depends_on = [google_project_service.secret_manager]
}

resource "google_secret_manager_secret" "polygon_api_key" {
  secret_id = "polygon-api-key"
  project   = var.project_id

  labels = {
    app        = "top200-rs"
    managed-by = "terraform"
  }

  replication {
    auto {}
  }

  depends_on = [google_project_service.secret_manager]
}

resource "google_secret_manager_secret" "anthropic_api_key" {
  secret_id = "anthropic-api-key"
  project   = var.project_id

  labels = {
    app        = "top200-rs"
    managed-by = "terraform"
  }

  replication {
    auto {}
  }

  depends_on = [google_project_service.secret_manager]
}

resource "google_secret_manager_secret" "brevo_api_key" {
  secret_id = "brevo-api-key"
  project   = var.project_id

  labels = {
    app        = "top200-rs"
    managed-by = "terraform"
  }

  replication {
    auto {}
  }

  depends_on = [google_project_service.secret_manager]
}

# Optional: Create Cloud Storage bucket for data and exports
resource "google_storage_bucket" "top200_data" {
  name     = "${var.project_id}-top200-data"
  location = var.region
  project  = var.project_id

  uniform_bucket_level_access = true

  labels = {
    app        = "top200-rs"
    managed-by = "terraform"
  }

  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      age = 90
    }
    action {
      type = "Delete"
    }
  }
}

# Grant storage access to service account
resource "google_storage_bucket_iam_member" "top200_data_admin" {
  bucket = google_storage_bucket.top200_data.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.top200_rs.email}"
}

# Optional: Create Artifact Registry for container images
resource "google_artifact_registry_repository" "top200_rs" {
  location      = var.region
  repository_id = "top200-rs"
  description   = "Container images for top200-rs"
  format        = "DOCKER"
  project       = var.project_id

  labels = {
    app        = "top200-rs"
    managed-by = "terraform"
  }

  depends_on = [google_project_service.artifact_registry]
}

# Grant Artifact Registry access to service account
resource "google_artifact_registry_repository_iam_member" "top200_rs_reader" {
  project    = var.project_id
  location   = google_artifact_registry_repository.top200_rs.location
  repository = google_artifact_registry_repository.top200_rs.name
  role       = "roles/artifactregistry.reader"
  member     = "serviceAccount:${google_service_account.top200_rs.email}"
}

# Grant Cloud Build service account permission to push images
data "google_project" "project" {
  project_id = var.project_id
}

resource "google_artifact_registry_repository_iam_member" "cloudbuild_writer" {
  project    = var.project_id
  location   = google_artifact_registry_repository.top200_rs.location
  repository = google_artifact_registry_repository.top200_rs.name
  role       = "roles/artifactregistry.writer"
  member     = "serviceAccount:${data.google_project.project.number}@cloudbuild.gserviceaccount.com"

  depends_on = [google_project_service.cloud_build]
}
