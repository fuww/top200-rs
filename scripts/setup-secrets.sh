#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Setup Google Secret Manager secrets for top200-rs
# This script creates secrets in Google Secret Manager and optionally sets their values

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if required tools are installed
check_requirements() {
    if ! command -v gcloud &> /dev/null; then
        echo -e "${RED}Error: gcloud CLI is not installed${NC}"
        echo "Please install it from: https://cloud.google.com/sdk/docs/install"
        exit 1
    fi
}

# Get GCP project ID
get_project_id() {
    if [ -n "${GCP_PROJECT_ID:-}" ]; then
        echo "$GCP_PROJECT_ID"
    else
        gcloud config get-value project 2>/dev/null || {
            echo -e "${RED}Error: No GCP project configured${NC}"
            echo "Please run: gcloud config set project YOUR_PROJECT_ID"
            exit 1
        }
    fi
}

# Create a secret if it doesn't exist
create_secret() {
    local secret_name=$1
    local project_id=$2

    if gcloud secrets describe "$secret_name" --project="$project_id" &>/dev/null; then
        echo -e "${YELLOW}Secret '$secret_name' already exists${NC}"
        return 0
    else
        echo -e "${GREEN}Creating secret '$secret_name'...${NC}"
        gcloud secrets create "$secret_name" \
            --project="$project_id" \
            --replication-policy="automatic" \
            --labels="app=top200-rs,managed-by=script"
    fi
}

# Set secret value
set_secret_value() {
    local secret_name=$1
    local secret_value=$2
    local project_id=$3

    echo -e "${GREEN}Setting value for secret '$secret_name'...${NC}"
    echo -n "$secret_value" | gcloud secrets versions add "$secret_name" \
        --project="$project_id" \
        --data-file=-
}

# Main setup function
main() {
    echo "==================================="
    echo "Google Secret Manager Setup"
    echo "==================================="
    echo ""

    check_requirements

    PROJECT_ID=$(get_project_id)
    echo -e "Using GCP Project: ${GREEN}$PROJECT_ID${NC}"
    echo ""

    # Enable Secret Manager API if not already enabled
    echo "Enabling Secret Manager API..."
    gcloud services enable secretmanager.googleapis.com --project="$PROJECT_ID" 2>/dev/null || true
    echo ""

    # List of secrets to create
    SECRETS=(
        "financialmodelingprep-api-key"
        "polygon-api-key"
        "anthropic-api-key"
        "brevo-api-key"
    )

    # Create all secrets
    for secret in "${SECRETS[@]}"; do
        create_secret "$secret" "$PROJECT_ID"
    done

    echo ""
    echo "==================================="
    echo "Secrets created successfully!"
    echo "==================================="
    echo ""

    # Ask if user wants to set values now
    read -p "Do you want to set secret values now? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""

        # Financial Modeling Prep API Key
        if [ -n "${FINANCIALMODELINGPREP_API_KEY:-}" ]; then
            echo "Using FINANCIALMODELINGPREP_API_KEY from environment"
            set_secret_value "financialmodelingprep-api-key" "$FINANCIALMODELINGPREP_API_KEY" "$PROJECT_ID"
        else
            read -sp "Enter Financial Modeling Prep API Key (or press Enter to skip): " fmp_key
            echo
            if [ -n "$fmp_key" ]; then
                set_secret_value "financialmodelingprep-api-key" "$fmp_key" "$PROJECT_ID"
            fi
        fi

        # Polygon API Key
        if [ -n "${POLYGON_API_KEY:-}" ]; then
            echo "Using POLYGON_API_KEY from environment"
            set_secret_value "polygon-api-key" "$POLYGON_API_KEY" "$PROJECT_ID"
        else
            read -sp "Enter Polygon API Key (or press Enter to skip): " polygon_key
            echo
            if [ -n "$polygon_key" ]; then
                set_secret_value "polygon-api-key" "$polygon_key" "$PROJECT_ID"
            fi
        fi

        # Anthropic API Key
        if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
            echo "Using ANTHROPIC_API_KEY from environment"
            set_secret_value "anthropic-api-key" "$ANTHROPIC_API_KEY" "$PROJECT_ID"
        else
            read -sp "Enter Anthropic API Key (or press Enter to skip): " anthropic_key
            echo
            if [ -n "$anthropic_key" ]; then
                set_secret_value "anthropic-api-key" "$anthropic_key" "$PROJECT_ID"
            fi
        fi

        # Brevo API Key
        if [ -n "${BREVO_API_KEY:-}" ]; then
            echo "Using BREVO_API_KEY from environment"
            set_secret_value "brevo-api-key" "$BREVO_API_KEY" "$PROJECT_ID"
        else
            read -sp "Enter Brevo API Key (or press Enter to skip): " brevo_key
            echo
            if [ -n "$brevo_key" ]; then
                set_secret_value "brevo-api-key" "$brevo_key" "$PROJECT_ID"
            fi
        fi

        echo ""
        echo -e "${GREEN}Secret values set successfully!${NC}"
    else
        echo ""
        echo "Skipping secret value setup."
        echo "You can set values later using:"
        echo "  echo -n 'YOUR_VALUE' | gcloud secrets versions add SECRET_NAME --data-file=-"
    fi

    echo ""
    echo "==================================="
    echo "Next Steps"
    echo "==================================="
    echo ""
    echo "1. Grant your service account access to the secrets:"
    echo "   gcloud projects add-iam-policy-binding $PROJECT_ID \\"
    echo "     --member='serviceAccount:YOUR_SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com' \\"
    echo "     --role='roles/secretmanager.secretAccessor'"
    echo ""
    echo "2. Set the GCP_PROJECT_ID environment variable:"
    echo "   export GCP_PROJECT_ID=$PROJECT_ID"
    echo ""
    echo "3. For local development with Application Default Credentials:"
    echo "   gcloud auth application-default login"
    echo ""
    echo "4. Or use a service account key file:"
    echo "   export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json"
    echo ""
}

main "$@"
