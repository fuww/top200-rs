// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, Validation, decode};

use crate::web::{
    models::auth::{Claims, User},
    state::AppState,
};

/// Middleware extractor for authenticated users
pub struct AuthUser(pub User);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract token from Authorization header or cookie
        let token = extract_token(parts)?;

        // Validate JWT
        let claims = validate_jwt(&token, &state.jwt_secret)?;

        // Convert claims to User
        let user = User::from_claims(&claims).ok_or(AuthError::InvalidRole)?;

        Ok(AuthUser(user))
    }
}

/// Extract JWT token from Authorization header or cookie
fn extract_token(parts: &Parts) -> Result<String, AuthError> {
    // Try Authorization header first (Bearer token)
    if let Some(auth_header) = parts.headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Ok(token.to_string());
            }
        }
    }

    // Try cookie
    if let Some(cookie_header) = parts.headers.get("cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split("; ") {
                if let Some(token) = cookie.strip_prefix("token=") {
                    return Ok(token.to_string());
                }
            }
        }
    }

    Err(AuthError::MissingToken)
}

/// Validate JWT token and extract claims
fn validate_jwt(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|_| AuthError::InvalidToken)
}

/// Authentication errors
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    InvalidRole,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
            AuthError::InvalidRole => (StatusCode::UNAUTHORIZED, "Invalid user role"),
        };

        (status, message).into_response()
    }
}
