// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use askama::Template;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Deserialize;
use workos::sso::{
    GetAuthorizationUrl, GetAuthorizationUrlParams,
    GetProfileAndToken, GetProfileAndTokenParams,
    AuthorizationCode, ClientId, ConnectionSelector, Provider,
};

use crate::web::{models::auth::Claims, state::AppState};

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    authorization_url: String,
    error: Option<String>,
}

/// Login page - shows WorkOS authorization button
pub async fn login_page(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    // Get configuration from environment
    let client_id = std::env::var("WORKOS_CLIENT_ID")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let redirect_uri = std::env::var("WORKOS_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/api/auth/callback".to_string());

    // Generate WorkOS authorization URL using Google OAuth provider
    let provider = Provider::GoogleOauth;
    let params = GetAuthorizationUrlParams {
        client_id: &ClientId::from(client_id.as_str()),
        redirect_uri: &redirect_uri,
        connection_selector: ConnectionSelector::Provider(&provider),
        state: None,
    };

    let authorization_url = state
        .workos_client
        .sso()
        .get_authorization_url(&params)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = LoginTemplate {
        authorization_url: authorization_url.to_string(),
        error: None,
    };

    Ok(Html(
        template.render().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[derive(Deserialize)]
pub struct AuthCallback {
    code: String,
}

/// WorkOS callback handler - exchanges code for user info and creates JWT
pub async fn auth_callback(
    Query(callback): Query<AuthCallback>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    // Get client ID from environment
    let client_id = std::env::var("WORKOS_CLIENT_ID")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Exchange authorization code for profile and token
    let params = GetProfileAndTokenParams {
        client_id: &ClientId::from(client_id.as_str()),
        code: &AuthorizationCode::from(callback.code.as_str()),
    };

    let response = state
        .workos_client
        .sso()
        .get_profile_and_token(&params)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let profile = response.profile;

    // Determine user role (default to Viewer, check if admin)
    // In production, you'd check this against a database or WorkOS organization roles
    let role = if is_admin_email(&profile.email) {
        "admin"
    } else {
        "viewer"
    };

    // Create JWT claims
    let now = Utc::now();
    let claims = Claims {
        sub: profile.id.to_string(),
        email: profile.email.clone(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: (now + Duration::days(7)).timestamp(),
    };

    // Encode JWT
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set cookie and redirect to dashboard
    let cookie = format!(
        "token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        token,
        60 * 60 * 24 * 7 // 7 days
    );

    Ok((
        StatusCode::SEE_OTHER,
        [(header::SET_COOKIE, cookie), (header::LOCATION, "/".to_string())],
    )
        .into_response())
}

/// Logout handler - clears cookie and redirects to login
pub async fn logout() -> impl IntoResponse {
    let cookie = "token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0".to_string();

    (
        StatusCode::SEE_OTHER,
        [(header::SET_COOKIE, cookie), (header::LOCATION, "/login".to_string())],
    )
}

/// Helper function to determine if an email should have admin role
/// In production, this should check against a database or WorkOS directory
fn is_admin_email(email: &str) -> bool {
    // Simple example: check if email is in admin list
    // In production, use database or WorkOS directory roles
    let admin_emails_env = std::env::var("ADMIN_EMAILS").unwrap_or_default();
    let admin_emails: Vec<_> = admin_emails_env
        .split(',')
        .map(|s| s.trim())
        .collect();

    admin_emails.contains(&email)
}
