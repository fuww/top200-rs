use super::*;
use crate::db;
use crate::models::Details;
use std::collections::HashMap;
use chrono::Utc;
use mockall::predicate::*;
use crate::api::MockFMPClientTrait;

#[tokio::test]
async fn test_market_cap_operations() -> Result<()> {
    let pool = db::create_test_pool().await?;
    db::migrate(&pool).await?;

    // Create test data
    let mut rate_map = HashMap::new();
    rate_map.insert("USD/EUR".to_string(), 0.85);
    let timestamp = Utc::now().timestamp();

    let details = Details {
        ticker: "TEST".to_string(),
        currency: "USD".to_string(),
        market_cap: Some(1000000.0),
        exchange: Some("NYSE".to_string()),
        name: Some("Test Company".to_string()),
        sector: Some("Technology".to_string()),
        industry: Some("Software".to_string()),
        country: Some("US".to_string()),
        isin: Some("US1234567890".to_string()),
        website: Some("https://test.com".to_string()),
        description: Some("Test description".to_string()),
        employees: Some("1000".to_string()),
    };

    // Test storing market cap
    store_market_cap(&pool, &details, &rate_map, timestamp).await?;

    // Test getting market caps
    let market_caps = get_market_caps(&pool).await?;
    assert!(!market_caps.is_empty());
    assert_eq!(market_caps[0].0, 1000000.0); // USD value
    assert_eq!(market_caps[0].1[0], "TEST");

    Ok(())
}

#[tokio::test]
async fn test_marketcaps_main() -> Result<()> {
    let pool = db::create_test_pool().await?;
    db::migrate(&pool).await?;

    // Set up mock FMP client
    let mut mock_client = MockFMPClientTrait::new();
    mock_client.expect_get_details()
        .returning(|_, _| {
            Ok(Details {
                ticker: "TEST".to_string(),
                currency: "USD".to_string(),
                market_cap: Some(1000000.0),
                exchange: Some("NYSE".to_string()),
                name: Some("Test Company".to_string()),
                sector: Some("Technology".to_string()),
                industry: Some("Software".to_string()),
                country: Some("US".to_string()),
                isin: Some("US1234567890".to_string()),
                website: Some("https://test.com".to_string()),
                description: Some("Test description".to_string()),
                employees: Some("1000".to_string()),
            })
        });

    // Test export functions
    let details = Details {
        ticker: "TEST".to_string(),
        currency: "USD".to_string(),
        market_cap: Some(1000000.0),
        exchange: Some("NYSE".to_string()),
        name: Some("Test Company".to_string()),
        sector: Some("Technology".to_string()),
        industry: Some("Software".to_string()),
        country: Some("US".to_string()),
        isin: Some("US1234567890".to_string()),
        website: Some("https://test.com".to_string()),
        description: Some("Test description".to_string()),
        employees: Some("1000".to_string()),
    };

    let mut rate_map = HashMap::new();
    rate_map.insert("USD/EUR".to_string(), 0.85);
    let timestamp = Utc::now().timestamp();

    store_market_cap(&pool, &details, &rate_map, timestamp).await?;
    export_market_caps(&pool).await?;
    export_top_100_active(&pool).await?;

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let pool = db::create_test_pool().await?;
    db::migrate(&pool).await?;

    // Test handling missing market cap
    let details = Details {
        ticker: "TEST".to_string(),
        currency: "USD".to_string(),
        market_cap: None, // Missing market cap
        exchange: Some("NYSE".to_string()),
        name: Some("Test Company".to_string()),
        sector: Some("Technology".to_string()),
        industry: Some("Software".to_string()),
        country: Some("US".to_string()),
        isin: Some("US1234567890".to_string()),
        website: Some("https://test.com".to_string()),
        description: Some("Test description".to_string()),
        employees: Some("1000".to_string()),
    };

    let mut rate_map = HashMap::new();
    rate_map.insert("USD/EUR".to_string(), 0.85);
    let timestamp = Utc::now().timestamp();

    // Should handle missing market cap gracefully
    let result = store_market_cap(&pool, &details, &rate_map, timestamp).await;
    assert!(result.is_err());

    // Test handling invalid currency conversion
    let details_invalid_currency = Details {
        ticker: "TEST2".to_string(),
        currency: "INVALID".to_string(), // Invalid currency
        market_cap: Some(1000000.0),
        exchange: Some("NYSE".to_string()),
        name: Some("Test Company".to_string()),
        sector: Some("Technology".to_string()),
        industry: Some("Software".to_string()),
        country: Some("US".to_string()),
        isin: Some("US1234567890".to_string()),
        website: Some("https://test.com".to_string()),
        description: Some("Test description".to_string()),
        employees: Some("1000".to_string()),
    };

    let result = store_market_cap(&pool, &details_invalid_currency, &rate_map, timestamp).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_update_market_caps_with_mock() -> Result<()> {
    let pool = db::create_test_pool().await?;
    db::migrate(&pool).await?;

    // Set up mock FMP client with both success and failure cases
    let mut mock_client = MockFMPClientTrait::new();
    
    // Success case
    mock_client.expect_get_details()
        .with(eq("SUCCESS"), always())
        .returning(|_, _| {
            Ok(Details {
                ticker: "SUCCESS".to_string(),
                currency: "USD".to_string(),
                market_cap: Some(1000000.0),
                exchange: Some("NYSE".to_string()),
                name: Some("Test Company".to_string()),
                sector: Some("Technology".to_string()),
                industry: Some("Software".to_string()),
                country: Some("US".to_string()),
                isin: Some("US1234567890".to_string()),
                website: Some("https://test.com".to_string()),
                description: Some("Test description".to_string()),
                employees: Some("1000".to_string()),
            })
        });

    // Failure case
    mock_client.expect_get_details()
        .with(eq("FAIL"), always())
        .returning(|_, _| {
            Err(anyhow::anyhow!("API error"))
        });

    // Test both successful and failed updates
    let tickers = vec!["SUCCESS".to_string(), "FAIL".to_string()];
    let rate_map = HashMap::new();
    let timestamp = Utc::now().timestamp();

    // Process tickers
    for ticker in &tickers {
        match mock_client.get_details(ticker, &rate_map).await {
            Ok(details) => {
                let _ = store_market_cap(&pool, &details, &rate_map, timestamp).await;
            }
            Err(e) => {
                eprintln!("Failed to fetch details for {}: {}", ticker, e);
            }
        }
    }

    // Verify results
    let market_caps = get_market_caps(&pool).await?;
    assert!(!market_caps.is_empty());
    assert!(market_caps.iter().any(|(_, tickers)| tickers.contains(&"SUCCESS".to_string())));
    assert!(!market_caps.iter().any(|(_, tickers)| tickers.contains(&"FAIL".to_string())));

    Ok(())
}
