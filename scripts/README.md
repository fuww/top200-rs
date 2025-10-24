# Scripts

This directory contains utility scripts for managing the top200-rs project.

## setup-secrets.sh

Setup script for Google Secret Manager integration.

### Usage

```bash
# Run interactively
./scripts/setup-secrets.sh

# Or with environment variables
export GCP_PROJECT_ID=your-project-id
export FINANCIALMODELINGPREP_API_KEY=your-fmp-key
export POLYGON_API_KEY=your-polygon-key
export ANTHROPIC_API_KEY=your-anthropic-key
export BREVO_API_KEY=your-brevo-key
./scripts/setup-secrets.sh
```

### What it does

1. Enables the Google Secret Manager API
2. Creates the following secrets:
   - `financialmodelingprep-api-key`
   - `polygon-api-key`
   - `anthropic-api-key`
   - `brevo-api-key`
3. Optionally sets secret values from environment variables or interactive prompts
4. Provides instructions for granting access to service accounts

### Prerequisites

- gcloud CLI installed and configured
- Appropriate GCP permissions to create secrets
- A GCP project selected or GCP_PROJECT_ID environment variable set

### Service Account Permissions

After creating secrets, grant your service account access:

```bash
# For GKE workload identity
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member='serviceAccount:YOUR_SERVICE_ACCOUNT@YOUR_PROJECT_ID.iam.gserviceaccount.com' \
  --role='roles/secretmanager.secretAccessor'

# For local development
gcloud auth application-default login
```

### Manual Secret Management

```bash
# Create a secret
gcloud secrets create my-secret --replication-policy="automatic"

# Add a secret version
echo -n "my-secret-value" | gcloud secrets versions add my-secret --data-file=-

# View secret value
gcloud secrets versions access latest --secret="my-secret"

# List all secrets
gcloud secrets list

# Delete a secret
gcloud secrets delete my-secret
```
