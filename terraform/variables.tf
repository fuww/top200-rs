# SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
#
# SPDX-License-Identifier: AGPL-3.0-only

variable "project_id" {
  description = "The GCP project ID"
  type        = string
}

variable "region" {
  description = "The GCP region for resources"
  type        = string
  default     = "europe-west1"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "prod"
}
