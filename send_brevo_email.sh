#!/bin/bash

# Brevo email notification script using jq for proper JSON construction
# This ensures the JSON is always valid regardless of special characters
#
# Usage: ./send_brevo_email.sh [workflow_type]
# workflow_type: "daily" or "multi-period" (defaults to "multi-period")

WORKFLOW_TYPE="${1:-multi-period}"

echo "Preparing email notification for workflow type: ${WORKFLOW_TYPE}"

# Validate email configuration
if [ -z "${BREVO_API_KEY}" ] || [ -z "${BREVO_SENDER_EMAIL}" ] || [ -z "${NOTIFICATION_RECIPIENTS}" ]; then
  echo "Error: Email configuration is incomplete."
  echo "Required: BREVO_API_KEY, BREVO_SENDER_EMAIL, NOTIFICATION_RECIPIENTS"
  echo "BREVO_API_KEY: $([ -n "${BREVO_API_KEY}" ] && echo "configured" || echo "missing")"
  echo "BREVO_SENDER_EMAIL: $([ -n "${BREVO_SENDER_EMAIL}" ] && echo "configured" || echo "missing")"
  echo "NOTIFICATION_RECIPIENTS: $([ -n "${NOTIFICATION_RECIPIENTS}" ] && echo "configured" || echo "missing")"
  exit 1
fi

# Parse recipients from environment variable (comma-separated)
IFS=',' read -ra RECIPIENTS <<< "$NOTIFICATION_RECIPIENTS"

# Build recipients array using jq (trim whitespace for each email)
RECIPIENTS_JSON=$(printf '%s\n' "${RECIPIENTS[@]}" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | jq -R -s 'split("\n") | map(select(length > 0)) | map({email: .})')

# Set HTML content based on workflow type
if [ "${WORKFLOW_TYPE}" = "daily" ]; then
  HTML_CONTENT="<html><body>
    <h2>Daily Data Collection Complete</h2>
    <p>The daily data collection job for Top200-RS has finished successfully.</p>
    
    <h3>Run Details:</h3>
    <ul>
      <li>Run Number: #${GITHUB_RUN_NUMBER}</li>
      <li>Commit: ${GITHUB_SHA}</li>
      <li>Workflow URL: <a href=\"${GITHUB_RUN_URL}\">View Workflow Run</a></li>
    </ul>
    
    <p>The generated CSV files are available as artifacts in the workflow run and will be retained for 90 days.</p>
  </body></html>"
else
  # Multi-period analysis workflow
  # Build download URLs for artifacts
  NIGHTLY_LINK_URL="https://nightly.link/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}/${ARTIFACT_NAME}.zip"
  ARTIFACTS_PAGE_URL="${GITHUB_RUN_URL}#artifacts"

  HTML_CONTENT="<html><body>
    <h2>Market Caps Multi-Period Analysis Complete</h2>
    <p>The daily market caps analysis for Top200-RS has finished successfully.</p>

    <h3>Analysis Periods:</h3>
    <ul>
      <li><strong>Current Date:</strong> ${TODAY_DATE}</li>
      <li><strong>7 Days Ago:</strong> ${WEEK_AGO_DATE}</li>
      <li><strong>Same Day Last Month:</strong> ${MONTH_AGO_DATE}</li>
    </ul>

    <h3>Comparisons Generated:</h3>
    <ul>
      <li>Week-over-week comparison (${WEEK_AGO_DATE} to ${TODAY_DATE})</li>
      <li>Month-over-month comparison (${MONTH_AGO_DATE} to ${TODAY_DATE})</li>
    </ul>

    <h3>Generated Outputs:</h3>
    <ul>
      <li>Market cap CSV files for all three dates</li>
      <li>Comparison CSV and summary reports</li>
      <li>Visualization charts (SVG format)</li>
    </ul>

    <h3 style=\"color: #0366d6;\">Download Files:</h3>
    <p style=\"margin-bottom: 10px;\">
      <a href=\"${NIGHTLY_LINK_URL}\" style=\"display: inline-block; padding: 10px 20px; background-color: #28a745; color: white; text-decoration: none; border-radius: 5px; font-weight: bold;\">Download All Files (ZIP)</a>
    </p>
    <p style=\"font-size: 0.9em; color: #666;\">
      Direct download link - no GitHub login required.<br/>
      Alternative: <a href=\"${ARTIFACTS_PAGE_URL}\">View on GitHub Artifacts</a>
    </p>

    <h3>Run Details:</h3>
    <ul>
      <li>Run Number: #${GITHUB_RUN_NUMBER}</li>
      <li>Commit: ${GITHUB_SHA}</li>
      <li>Workflow URL: <a href=\"${GITHUB_RUN_URL}\">View Workflow Run</a></li>
    </ul>

    <p>All generated files will be retained for 90 days.</p>
  </body></html>"
fi

# Build the complete JSON payload using jq
JSON_PAYLOAD=$(jq -n \
  --arg sender_name "${BREVO_SENDER_NAME}" \
  --arg sender_email "${BREVO_SENDER_EMAIL}" \
  --argjson recipients "${RECIPIENTS_JSON}" \
  --arg subject "${EMAIL_SUBJECT}" \
  --arg html_content "${HTML_CONTENT}" \
  '{
    sender: {
      name: $sender_name,
      email: $sender_email
    },
    to: $recipients,
    subject: $subject,
    htmlContent: $html_content
  }')

echo "Sending email..."
echo "JSON payload:"
echo "$JSON_PAYLOAD" | jq .

# Send request
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST https://api.brevo.com/v3/smtp/email \
  -H "accept: application/json" \
  -H "api-key: ${BREVO_API_KEY}" \
  -H "content-type: application/json" \
  -d "${JSON_PAYLOAD}")

# Extract response code and body
RESPONSE_CODE=$(echo "$RESPONSE" | tail -n 1)
RESPONSE_BODY=$(echo "$RESPONSE" | head -n -1)

echo "Brevo API response code: ${RESPONSE_CODE}"
if [ "${RESPONSE_CODE}" -eq 201 ]; then
  echo "Email notification sent successfully."
else
  echo "Failed to send email notification. Response code: ${RESPONSE_CODE}"
  echo "Response body: ${RESPONSE_BODY}"
  exit 1
fi