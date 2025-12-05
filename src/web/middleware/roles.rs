// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};

use crate::web::{
    middleware::auth::AuthUser,
    models::auth::Role,
    state::AppState,
};

/// Middleware extractor that requires admin role
pub struct RequireAdmin(pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = RoleError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state)
            .await
            .map_err(|_| RoleError::Unauthorized)?;

        if user.role != Role::Admin {
            return Err(RoleError::Forbidden);
        }

        Ok(RequireAdmin(AuthUser(user)))
    }
}

/// Middleware extractor that requires at least viewer role (Admin or Viewer)
pub struct RequireViewer(pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for RequireViewer {
    type Rejection = RoleError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state)
            .await
            .map_err(|_| RoleError::Unauthorized)?;

        // Both Admin and Viewer can access viewer-protected routes
        Ok(RequireViewer(auth_user))
    }
}

/// Role-based authorization errors
#[derive(Debug)]
pub enum RoleError {
    Unauthorized,
    Forbidden,
}

impl IntoResponse for RoleError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            RoleError::Unauthorized => (StatusCode::UNAUTHORIZED, "Authentication required"),
            RoleError::Forbidden => (StatusCode::FORBIDDEN, "Insufficient permissions"),
        };

        (status, message).into_response()
    }
}
