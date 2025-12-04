// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use sqlx::SqlitePool;

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: Config,
}

impl AppState {
    pub fn new(db_pool: SqlitePool, config: Config) -> Self {
        Self { db_pool, config }
    }
}
