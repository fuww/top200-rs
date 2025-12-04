// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use askama::Template;
use axum::{extract::State, response::Html};

use crate::web::state::AppState;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    title: String,
}

/// Dashboard page handler
pub async fn dashboard(State(_state): State<AppState>) -> Html<String> {
    let template = DashboardTemplate {
        title: "Dashboard".to_string(),
    };
    Html(template.render().unwrap())
}
