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

#[derive(Debug, Serialize, Deserialize)]
pub struct FMPRatios {
    pub symbol: String,
    #[serde(rename = "currentRatio")]
    pub current_ratio: Option<f64>,
    #[serde(rename = "quickRatio")]
    pub quick_ratio: Option<f64>,
    #[serde(rename = "eps")]
    pub eps: Option<f64>,
    #[serde(rename = "priceEarningsRatio")]
    pub price_earnings_ratio: Option<f64>,
    #[serde(rename = "debtEquityRatio")]
    pub debt_equity_ratio: Option<f64>,
    #[serde(rename = "returnOnEquity")]
    pub return_on_equity: Option<f64>,
    // Add catch-all for other fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FMPIncomeStatement {
    pub symbol: String,
    pub date: String,
    #[serde(rename = "revenue")]
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

fn default_string() -> String {
    "0".to_string()
}
