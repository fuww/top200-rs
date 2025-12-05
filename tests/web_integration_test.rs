// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the web interface
//!
//! These tests verify that the web server responds correctly to requests
//! and that the UI pages render without errors.

use chrono::Utc;
use reqwest;
use std::time::Duration;
use tokio;

const BASE_URL: &str = "http://localhost:3001";

/// Test that the server is running and health check works
#[tokio::test]
async fn test_health_check() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(json["status"], "ok");
    assert!(json["timestamp"].is_string());
}

/// Test that the dashboard page loads
#[tokio::test]
async fn test_dashboard_loads() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let html = response.text().await.expect("Failed to get HTML");
    assert!(html.contains("Top200-rs"));
    assert!(html.contains("Welcome to Top200-rs"));
    assert!(html.contains("Quick Actions"));
}

/// Test that the comparisons list page loads
#[tokio::test]
async fn test_comparisons_list_loads() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/comparisons", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let html = response.text().await.expect("Failed to get HTML");
    assert!(html.contains("Comparisons"));
}

/// Test that the new comparison form page loads
#[tokio::test]
async fn test_new_comparison_form_loads() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/comparisons/new", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let html = response.text().await.expect("Failed to get HTML");
    assert!(html.contains("Generate Comparison"));
    assert!(html.contains("From Date"));
    assert!(html.contains("To Date"));
    assert!(html.contains("Generate visualization charts"));
}

/// Test that the market caps list page loads
#[tokio::test]
async fn test_market_caps_list_loads() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/market-caps", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let html = response.text().await.expect("Failed to get HTML");
    assert!(html.contains("Market Cap Snapshots"));
}

/// Test that the fetch market caps form page loads
#[tokio::test]
async fn test_fetch_market_caps_form_loads() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/market-caps/fetch", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let html = response.text().await.expect("Failed to get HTML");
    assert!(html.contains("Fetch Market Caps"));
    assert!(html.contains("Select the date for which to fetch market cap data"));
}

/// Test that the API comparisons endpoint returns valid JSON
#[tokio::test]
async fn test_api_comparisons_list() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/comparisons", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    // API returns {"comparisons": [...]}
    assert!(json.is_object());
    assert!(json.get("comparisons").is_some());
    let comparisons = json.get("comparisons").unwrap();
    assert!(comparisons.is_array());
}

/// Test that the API market caps endpoint returns valid JSON
#[tokio::test]
async fn test_api_market_caps_list() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/market-caps", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    // API returns {"snapshots": [...]}
    assert!(json.is_object());
    assert!(json.get("snapshots").is_some());
    let snapshots = json.get("snapshots").unwrap();
    assert!(snapshots.is_array());
}

/// Test that SSE endpoint for comparison generation is accessible
#[tokio::test]
async fn test_sse_comparison_endpoint_accessible() {
    let client = reqwest::Client::new();

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let yesterday = (Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let url = format!(
        "{}/api/generate-comparison-sse?from_date={}&to_date={}&generate_charts=false",
        BASE_URL, yesterday, today
    );

    // Just test that the endpoint is accessible
    // We won't wait for completion as it takes too long
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    // Either we get a response or timeout (both are acceptable for this test)
    match response {
        Ok(resp) => {
            // If we got a response, it should be 200
            assert_eq!(resp.status(), 200);
        }
        Err(e) => {
            // Timeout is acceptable for SSE endpoint
            if e.is_timeout() {
                // This is fine - SSE is streaming
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

/// Test that SSE endpoint for market caps fetch is accessible
#[tokio::test]
async fn test_sse_market_caps_endpoint_accessible() {
    let client = reqwest::Client::new();

    let today = Utc::now().format("%Y-%m-%d").to_string();

    let url = format!("{}/api/fetch-market-caps-sse?date={}", BASE_URL, today);

    // Just test that the endpoint is accessible
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    // Either we get a response or timeout (both are acceptable for this test)
    match response {
        Ok(resp) => {
            // If we got a response, it should be 200
            assert_eq!(resp.status(), 200);
        }
        Err(e) => {
            // Timeout is acceptable for SSE endpoint
            if e.is_timeout() {
                // This is fine - SSE is streaming
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

/// Test that static files are served correctly
#[tokio::test]
async fn test_static_css_loads() {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/static/css/output.css", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");

    // Should either return 200 (if CSS exists) or 404 (if not built yet)
    assert!(
        response.status() == 200 || response.status() == 404,
        "Expected 200 or 404, got {}",
        response.status()
    );

    if response.status() == 200 {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok());
        // Should be CSS
        if let Some(ct) = content_type {
            assert!(ct.contains("css") || ct.contains("text/plain"));
        }
    }
}

/// Test navigation between pages
#[tokio::test]
async fn test_page_navigation() {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build client");

    // Dashboard should load
    let response = client
        .get(format!("{}/", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");
    assert_eq!(response.status(), 200);

    // Comparisons list should load
    let response = client
        .get(format!("{}/comparisons", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");
    assert_eq!(response.status(), 200);

    // Market caps list should load
    let response = client
        .get(format!("{}/market-caps", BASE_URL))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Failed to connect to server");
    assert_eq!(response.status(), 200);
}

/// Test that invalid routes return 404
#[tokio::test]
async fn test_invalid_routes_return_404() {
    let client = reqwest::Client::new();

    let invalid_routes = vec![
        "/nonexistent",
        "/comparisons/invalid/invalid",
        "/market-caps/invalid-date",
        "/api/invalid",
    ];

    for route in invalid_routes {
        let response = client
            .get(format!("{}{}", BASE_URL, route))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .expect("Failed to connect to server");

        assert_eq!(response.status(), 404, "Route {} should return 404", route);
    }
}
