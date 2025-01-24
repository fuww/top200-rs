// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Details {
    pub ticker: String,
    #[serde(rename = "market_cap")]
    pub market_cap: Option<f64>,
    pub name: Option<String>,
    #[serde(rename = "currency_name")]
    pub currency_name: Option<String>,
    #[serde(rename = "currency_symbol")]
    pub currency_symbol: Option<String>,
    pub active: Option<bool>,
    pub description: Option<String>,
    #[serde(rename = "homepage_url")]
    pub homepage_url: Option<String>,
    #[serde(rename = "weighted_shares_outstanding")]
    pub weighted_shares_outstanding: Option<f64>,
    pub employees: Option<String>,
    pub revenue: Option<f64>,
    pub revenue_usd: Option<f64>,
    pub timestamp: Option<String>,
    // Financial ratios
    pub working_capital_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub eps: Option<f64>,
    pub pe_ratio: Option<f64>,
    pub debt_equity_ratio: Option<f64>,
    pub roe: Option<f64>,
    // Add catch-all for other fields we don't care about
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolygonResponse {
    pub status: String,
    pub request_id: String,
    pub results: Details,
    // Add catch-all for other fields we don't care about
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FMPCompanyProfile {
    pub symbol: String,
    #[serde(rename = "companyName")]
    pub company_name: String,
    #[serde(rename = "mktCap", default)]
    pub market_cap: f64,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "website", default)]
    pub website: String,
    #[serde(rename = "fullTimeEmployees")]
    pub employees: Option<String>,
    #[serde(rename = "price", default)]
    pub price: f64,
    pub currency: String,
    #[serde(rename = "exchangeShortName")]
    pub exchange: String,
    #[serde(rename = "isActivelyTrading", default)]
    pub is_active: bool,
    // Add any other fields you need from the FMP API
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct FMPRatios {
    pub symbol: String,
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub eps: Option<f64>,
    pub price_earnings_ratio: Option<f64>,
    pub debt_equity_ratio: Option<f64>,
    pub return_on_equity: Option<f64>,
    // Add catch-all for other fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct FMPIncomeStatement {
    pub date: String,
    pub symbol: String,
    pub revenue: Option<f64>,
    // Add catch-all for other fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stock {
    pub ticker: String,
    pub name: String,
    pub market_cap: f64,
    pub currency_name: String,
    pub currency_symbol: String,
    pub active: bool,
    pub description: String,
    pub homepage_url: String,
    pub employees: String,
    pub revenue: f64,
    pub revenue_usd: f64,
    pub working_capital_ratio: f64,
    pub quick_ratio: f64,
    pub eps: f64,
    pub pe_ratio: f64,
    pub debt_equity_ratio: f64,
    pub roe: f64,
}

#[allow(dead_code)]
fn default_string() -> String {
    "0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_details_serialization() {
        let details = Details {
            ticker: "AAPL".to_string(),
            market_cap: Some(2000000000000.0),
            name: Some("Apple Inc.".to_string()),
            currency_name: Some("US Dollar".to_string()),
            currency_symbol: Some("USD".to_string()),
            active: Some(true),
            description: Some("Technology company".to_string()),
            homepage_url: Some("https://www.apple.com".to_string()),
            weighted_shares_outstanding: Some(16000000000.0),
            employees: Some("100000".to_string()),
            revenue: Some(365000000000.0),
            revenue_usd: Some(365000000000.0),
            timestamp: Some("2024-01-01".to_string()),
            working_capital_ratio: Some(1.2),
            quick_ratio: Some(0.9),
            eps: Some(6.05),
            pe_ratio: Some(28.5),
            debt_equity_ratio: Some(2.1),
            roe: Some(0.15),
            extra: std::collections::HashMap::new(),
        };

        let json = serde_json::to_string(&details).unwrap();
        let deserialized: Details = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.ticker, "AAPL");
        assert_eq!(deserialized.market_cap, Some(2000000000000.0));
        assert_eq!(deserialized.name, Some("Apple Inc.".to_string()));
        assert_eq!(deserialized.currency_symbol, Some("USD".to_string()));
        assert_eq!(deserialized.active, Some(true));
    }

    #[test]
    fn test_polygon_response_deserialization() {
        let json = json!({
            "status": "OK",
            "request_id": "123",
            "results": {
                "ticker": "AAPL",
                "market_cap": 2000000000000.0,
                "name": "Apple Inc.",
                "currency_symbol": "USD",
                "active": true
            }
        });

        let response: PolygonResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.status, "OK");
        assert_eq!(response.request_id, "123");
        assert_eq!(response.results.ticker, "AAPL");
        assert_eq!(response.results.market_cap, Some(2000000000000.0));
    }

    #[test]
    fn test_fmp_company_profile_deserialization() {
        let json = json!({
            "symbol": "AAPL",
            "companyName": "Apple Inc.",
            "mktCap": 2000000000000.0,
            "description": "Technology company",
            "website": "https://www.apple.com",
            "fullTimeEmployees": "100000+",
            "price": 150.0,
            "currency": "USD",
            "exchangeShortName": "NASDAQ",
            "isActivelyTrading": true
        });

        let profile: FMPCompanyProfile = serde_json::from_value(json).unwrap();
        assert_eq!(profile.symbol, "AAPL");
        assert_eq!(profile.company_name, "Apple Inc.");
        assert_eq!(profile.market_cap, 2000000000000.0);
        assert_eq!(profile.price, 150.0);
        assert_eq!(profile.currency, "USD");
        assert_eq!(profile.exchange, "NASDAQ");
        assert_eq!(profile.is_active, true);
    }

    #[test]
    fn test_fmp_ratios_deserialization() {
        let json = json!({
            "symbol": "AAPL",
            "current_ratio": 1.2,
            "quick_ratio": 0.9,
            "eps": 6.05,
            "price_earnings_ratio": 28.5,
            "debt_equity_ratio": 2.1,
            "return_on_equity": 0.15
        });

        let ratios: FMPRatios = serde_json::from_value(json).unwrap();
        assert_eq!(ratios.symbol, "AAPL");
        assert_eq!(ratios.current_ratio, Some(1.2));
        assert_eq!(ratios.quick_ratio, Some(0.9));
        assert_eq!(ratios.eps, Some(6.05));
        assert_eq!(ratios.price_earnings_ratio, Some(28.5));
        assert_eq!(ratios.debt_equity_ratio, Some(2.1));
        assert_eq!(ratios.return_on_equity, Some(0.15));
    }

    #[test]
    fn test_fmp_income_statement_deserialization() {
        let json = json!({
            "date": "2024-01-01",
            "symbol": "AAPL",
            "revenue": 365000000000.0
        });

        let statement: FMPIncomeStatement = serde_json::from_value(json).unwrap();
        assert_eq!(statement.date, "2024-01-01");
        assert_eq!(statement.symbol, "AAPL");
        assert_eq!(statement.revenue, Some(365000000000.0));
    }

    #[test]
    fn test_stock_serialization() {
        let stock = Stock {
            ticker: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            market_cap: 2000000000000.0,
            currency_name: "US Dollar".to_string(),
            currency_symbol: "USD".to_string(),
            active: true,
            description: "Technology company".to_string(),
            homepage_url: "https://www.apple.com".to_string(),
            employees: "100000+".to_string(),
            revenue: 365000000000.0,
            revenue_usd: 365000000000.0,
            working_capital_ratio: 1.2,
            quick_ratio: 0.9,
            eps: 6.05,
            pe_ratio: 28.5,
            debt_equity_ratio: 2.1,
            roe: 0.15,
        };

        let json = serde_json::to_string(&stock).unwrap();
        let deserialized: Stock = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.ticker, "AAPL");
        assert_eq!(deserialized.market_cap, 2000000000000.0);
        assert_eq!(deserialized.currency_symbol, "USD");
        assert_eq!(deserialized.active, true);
        assert_eq!(deserialized.revenue, 365000000000.0);
        assert_eq!(deserialized.eps, 6.05);
    }
}
