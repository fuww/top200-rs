# Email Notification Configuration

This document describes how to configure email notifications for the Top200-RS workflows.

## Environment Variables

The following environment variables control email notifications:

### Required Variables

- `BREVO_API_KEY`: Your Brevo API key for sending transactional emails
- `BREVO_SENDER_EMAIL`: The email address to send notifications from
- `NOTIFICATION_RECIPIENTS`: Comma-separated list of email addresses to send notifications to

### Optional Variables

- `BREVO_SENDER_NAME`: Display name for the sender (defaults to "Top200-RS Notifier")

## Setting Up Email Recipients

### Single Recipient

```bash
NOTIFICATION_RECIPIENTS=user@example.com
```

### Multiple Recipients

```bash
NOTIFICATION_RECIPIENTS=user1@example.com,user2@example.com,user3@example.com
```

Recipients can be separated by commas with or without spaces. The system automatically trims whitespace.

## GitHub Secrets and Variables

Configure these in your GitHub repository settings:

### Secrets (Sensitive Information)
- `BREVO_API_KEY`: Your Brevo API key
- `NOTIFICATION_RECIPIENTS`: Email addresses (if you want to keep them private)

### Variables (Non-Sensitive Information)
- `BREVO_SENDER_EMAIL`: Sender email address
- `BREVO_SENDER_NAME`: Sender display name
- `NOTIFICATION_RECIPIENTS`: Email addresses (alternative to secret)

## Email Content

The notification emails include:

1. **Workflow Status**: Success/failure notification
2. **Run Information**: Run number, commit SHA
3. **Direct Links**: 
   - Link to the workflow run
   - Direct link to download artifacts
4. **Context**: Specific information about the data collection job

## Artifact Links

Each email includes a direct link to the artifacts page where recipients can:
- View all generated files
- Download CSV reports
- Access logs and other workflow outputs

## Fallback Behavior

If `NOTIFICATION_RECIPIENTS` is not set, the workflows fall back to:
- `joost@fashionunited.com` for the daily-run workflow
- `lennard@fashionunited.com` for the daily-specific-date workflow

This ensures backward compatibility with existing deployments.