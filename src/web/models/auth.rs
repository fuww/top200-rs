// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// User roles for authorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::Admin => "admin",
            Role::Viewer => "viewer",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User ID from WorkOS
    pub sub: String,
    /// User email
    pub email: String,
    /// User role
    pub role: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: Role,
}

impl User {
    pub fn from_claims(claims: &Claims) -> Option<Self> {
        let role = Role::from_str(&claims.role)?;
        Some(Self {
            id: claims.sub.clone(),
            email: claims.email.clone(),
            name: None,
            role,
        })
    }

    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    pub fn is_viewer(&self) -> bool {
        self.role == Role::Viewer
    }
}
