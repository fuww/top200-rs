# Terraform Infrastructure for Top200-RS

This directory contains Terraform configuration to set up the Google Cloud Platform infrastructure for the top200-rs application.

## What Gets Created

- **Service Account**: `top200-rs@YOUR_PROJECT.iam.gserviceaccount.com`
- **Secrets** in Google Secret Manager:
  - `financialmodelingprep-api-key`
  - `polygon-api-key`
  - `anthropic-api-key`
  - `brevo-api-key`
- **Cloud Storage Bucket**: For storing data and exports
- **Artifact Registry**: For Docker container images
- **IAM Permissions**: Appropriate roles for the service account

## Prerequisites

1. **Google Cloud SDK** installed and configured:
   ```bash
   # Install gcloud CLI
   # See: https://cloud.google.com/sdk/docs/install

   # Authenticate
   gcloud auth login

   # Set your project
   gcloud config set project YOUR_PROJECT_ID
   ```

2. **Terraform** installed (version >= 1.0):
   ```bash
   # macOS
   brew install terraform

   # Or download from: https://www.terraform.io/downloads
   ```

3. **GCP Project** with billing enabled

4. **Appropriate permissions** in your GCP project:
   - Project Editor or Owner role
   - Or specific roles:
     - `roles/secretmanager.admin`
     - `roles/storage.admin`
     - `roles/iam.serviceAccountAdmin`
     - `roles/artifactregistry.admin`
     - `roles/serviceusage.serviceUsageAdmin`

## Initial Setup

1. **Create your tfvars file**:
   ```bash
   cd terraform
   cp terraform.tfvars.example terraform.tfvars
   # Edit terraform.tfvars with your project ID
   ```

2. **Enable required APIs** (optional, Terraform will do this):
   ```bash
   gcloud services enable cloudresourcemanager.googleapis.com
   gcloud services enable serviceusage.googleapis.com
   ```

3. **Initialize Terraform**:
   ```bash
   terraform init
   ```

## Usage

### Plan Infrastructure Changes

Preview what will be created:

```bash
terraform plan
```

### Apply Infrastructure

Create the infrastructure:

```bash
terraform apply
```

Review the plan and type `yes` to confirm.

### View Outputs

After applying, view the created resources:

```bash
terraform output
```

### Set Secret Values

After infrastructure is created, set the secret values:

```bash
# Option 1: Use the setup script
../scripts/setup-secrets.sh

# Option 2: Manually set each secret
echo -n "YOUR_FMP_API_KEY" | gcloud secrets versions add financialmodelingprep-api-key --data-file=-
echo -n "YOUR_POLYGON_KEY" | gcloud secrets versions add polygon-api-key --data-file=-
echo -n "YOUR_ANTHROPIC_KEY" | gcloud secrets versions add anthropic-api-key --data-file=-
echo -n "YOUR_BREVO_KEY" | gcloud secrets versions add brevo-api-key --data-file=-
```

### Destroy Infrastructure

To remove all created resources:

```bash
terraform destroy
```

**Warning**: This will delete all secrets, the storage bucket, and all data!

## State Management

### Local State (Default)

By default, Terraform stores state locally in `terraform.tfstate`. This is fine for single-user setups.

### Remote State (Recommended for Teams)

For team collaboration, use Google Cloud Storage as a backend:

1. **Create a state bucket**:
   ```bash
   gsutil mb -p YOUR_PROJECT_ID -l europe-west1 gs://YOUR_PROJECT_ID-terraform-state
   gsutil versioning set on gs://YOUR_PROJECT_ID-terraform-state
   ```

2. **Update main.tf** backend configuration:
   ```hcl
   terraform {
     backend "gcs" {
       bucket = "YOUR_PROJECT_ID-terraform-state"
       prefix = "terraform/top200-rs"
     }
   }
   ```

3. **Re-initialize**:
   ```bash
   terraform init -migrate-state
   ```

## Resource Naming

All resources are labeled with:
- `app = "top200-rs"`
- `managed-by = "terraform"`

This makes it easy to identify and manage resources.

## Service Account Usage

### Local Development

Create a key for local development:

```bash
# Get service account email from Terraform output
SA_EMAIL=$(terraform output -raw service_account_email)

# Create key
gcloud iam service-accounts keys create key.json \
  --iam-account=$SA_EMAIL

# Use the key
export GOOGLE_APPLICATION_CREDENTIALS=$PWD/key.json
export GCP_PROJECT_ID=YOUR_PROJECT_ID
```

**Important**: Never commit `key.json` to version control!

### Cloud Run / GKE

For Cloud Run or GKE, use Workload Identity instead of key files:

```bash
# Cloud Run example
gcloud run deploy top200-rs \
  --service-account=$SA_EMAIL \
  --region=europe-west1 \
  --image=europe-west1-docker.pkg.dev/PROJECT/top200-rs/top200-rs:latest
```

## Updating Infrastructure

1. **Modify Terraform files** as needed
2. **Plan changes**:
   ```bash
   terraform plan
   ```
3. **Apply changes**:
   ```bash
   terraform apply
   ```

## Troubleshooting

### "API not enabled" errors

Some APIs take a few minutes to enable. Wait and retry:

```bash
terraform apply -auto-approve
```

### Permission denied

Ensure you have the required roles:

```bash
gcloud projects get-iam-policy YOUR_PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:user:YOUR_EMAIL"
```

### State lock issues

If Terraform crashes, you may need to force-unlock:

```bash
terraform force-unlock LOCK_ID
```

## Security Best Practices

1. **Never commit**:
   - `terraform.tfvars` (contains project ID)
   - `*.tfstate` files (may contain sensitive data)
   - `key.json` service account keys

2. **Use least privilege**:
   - Service account has only necessary permissions
   - Secrets are only accessible to authorized accounts

3. **Enable audit logging**:
   ```bash
   gcloud logging read "resource.type=secret_manager_secret" --limit 50
   ```

4. **Rotate secrets regularly**:
   ```bash
   # Add new version
   echo -n "NEW_KEY" | gcloud secrets versions add SECRET_NAME --data-file=-

   # Disable old version
   gcloud secrets versions disable VERSION --secret=SECRET_NAME
   ```

## Cost Estimates

Approximate monthly costs (as of 2025):

- Secret Manager: ~$0.06 per secret per month + $0.03 per 10k accesses
- Cloud Storage: ~$0.02 per GB per month (europe-west1)
- Artifact Registry: First 0.5 GB free, then ~$0.10 per GB per month
- Service Account: Free

Total estimated cost: < $1/month for small usage

## Additional Resources

- [Google Secret Manager Documentation](https://cloud.google.com/secret-manager/docs)
- [Terraform Google Provider](https://registry.terraform.io/providers/hashicorp/google/latest/docs)
- [GCP Best Practices for Secrets](https://cloud.google.com/secret-manager/docs/best-practices)
