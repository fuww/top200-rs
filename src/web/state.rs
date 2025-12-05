// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use sqlx::SqlitePool;
use workos::WorkOs;

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: Config,
    pub workos_client: WorkOs,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(
        db_pool: SqlitePool,
        config: Config,
        workos_client: WorkOs,
        jwt_secret: String,
    ) -> Self {
        Self {
            db_pool,
            config,
            workos_client,
            jwt_secret,
        }
    }
}
