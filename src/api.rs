use anyhow::{Context, Result};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{self, Value};
use std::{env, time::Duration};
use tokio::time::sleep;
use std::collections::HashMap;

use crate::models::{Details, PolygonResponse, FMPCompanyProfile, FMPRatios, FMPIncomeStatement};
use crate::convert_currency;

pub struct PolygonClient {
    client: Client,
    api_key: String,
}

pub struct FMPClient {
    client: Client,
    api_key: String,
}

impl FMPClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn get_details(&self, ticker: &str, rate_map: &HashMap<String, f64>) -> Result<Details> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        // Add a small delay to stay within 300 calls/min limit
        sleep(Duration::from_millis(200)).await;

        let url = format!(
            "https://financialmodelingprep.com/api/v3/profile/{}?apikey={}",
            ticker,
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        let profiles: Vec<FMPCompanyProfile> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if profiles.is_empty() {
            anyhow::bail!("No data found for ticker");
        }

        let profile = &profiles[0];
        let currency = profile.currency.as_str();

        // Get current timestamp in ISO 8601 format
        let timestamp = chrono::Utc::now().to_rfc3339();

        let mut details = Details {
            ticker: profile.symbol.clone(),
            market_cap: Some(profile.market_cap),
            name: Some(profile.company_name.clone()),
            currency_name: Some(currency.to_string()),
            currency_symbol: Some(currency.to_string()),
            active: Some(profile.is_active),
            description: Some(profile.description.clone()),
            homepage_url: Some(profile.website.clone()),
            weighted_shares_outstanding: None,  // Not available in FMP API
            employees: profile.employees.clone(),
            revenue: None,
            revenue_usd: None,
            timestamp: Some(timestamp),
            working_capital_ratio: None,
            quick_ratio: None,
            eps: None,
            pe_ratio: None,
            debt_equity_ratio: None,
            roe: None,
            extra: {
                let mut map = std::collections::HashMap::new();
                map.insert("exchange".to_string(), Value::String(profile.exchange.clone()));
                map.insert("price".to_string(), Value::Number(serde_json::Number::from_f64(profile.price).unwrap_or(serde_json::Number::from(0))));
                map
            },
        };

        // Fetch ratios and income statement
        let ratios = self.get_ratios(ticker).await?;
        let income = self.get_income_statement(ticker).await?;

        // Calculate revenue in USD if available
        let revenue = income.as_ref().and_then(|i| i.revenue);
        let revenue_usd = revenue.map(|rev| {
            convert_currency(
                rev,
                &profile.currency,
                "USD",
                rate_map,
            )
        });

        details.revenue = revenue;
        details.revenue_usd = revenue_usd;
        details.working_capital_ratio = ratios.as_ref().and_then(|r| r.current_ratio);
        details.quick_ratio = ratios.as_ref().and_then(|r| r.quick_ratio);
        details.eps = ratios.as_ref().and_then(|r| r.eps);
        details.pe_ratio = ratios.as_ref().and_then(|r| r.price_earnings_ratio);
        details.debt_equity_ratio = ratios.as_ref().and_then(|r| r.debt_equity_ratio);
        details.roe = ratios.as_ref().and_then(|r| r.return_on_equity);

        Ok(details)
    }

    pub async fn get_ratios(&self, ticker: &str) -> Result<Option<FMPRatios>> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        // Add a small delay to stay within rate limit
        sleep(Duration::from_millis(200)).await;

        let url = format!(
            "https://financialmodelingprep.com/api/v3/ratios/{}?apikey={}",
            ticker,
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let text = response.text().await.context("Failed to get response text")?;

        if !status.is_success() {
            anyhow::bail!("API request failed: {}", text);
        }

        let ratios: Vec<FMPRatios> = serde_json::from_str(&text)
            .context("Failed to parse FMP ratios response")?;

        // Get the most recent ratios (first in the list)
        Ok(ratios.into_iter().next())
    }

    pub async fn get_income_statement(&self, ticker: &str) -> Result<Option<FMPIncomeStatement>> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        // Add a small delay to stay within rate limit
        sleep(Duration::from_millis(200)).await;

        let url = format!(
            "https://financialmodelingprep.com/api/v3/income-statement/{}?limit=1&apikey={}",
            ticker,
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let text = response.text().await.context("Failed to get response text")?;

        if !status.is_success() {
            anyhow::bail!("API request failed: {}", text);
        }

        let statements: Vec<FMPIncomeStatement> = serde_json::from_str(&text)
            .context("Failed to parse FMP income statement response")?;

        // Get the most recent statement (first in the list)
        Ok(statements.into_iter().next())
    }

    pub async fn get_exchange_rates(&self) -> Result<Vec<ExchangeRate>, Box<dyn std::error::Error>> {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/quotes/forex?apikey={}",
            self.api_key
        );

        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let rates: Vec<ExchangeRate> = response.json().await?;
        Ok(rates)
    }
}

#[derive(Debug, Deserialize)]
pub struct ExchangeRate {
    pub name: String,
    pub price: f64,
    #[serde(rename = "changesPercentage")]
    pub changes_percentage: Option<f64>,
    pub change: Option<f64>,
    #[serde(rename = "dayLow")]
    pub day_low: Option<f64>,
    #[serde(rename = "dayHigh")]
    pub day_high: Option<f64>,
    #[serde(rename = "yearHigh")]
    pub year_high: Option<f64>,
    #[serde(rename = "yearLow")]
    pub year_low: Option<f64>,
    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,
    #[serde(rename = "priceAvg50")]
    pub price_avg_50: Option<f64>,
    #[serde(rename = "priceAvg200")]
    pub price_avg_200: Option<f64>,
    pub volume: Option<f64>,
    #[serde(rename = "avgVolume")]
    pub avg_volume: Option<f64>,
    pub exchange: Option<String>,
    pub open: Option<f64>,
    #[serde(rename = "previousClose")]
    pub previous_close: Option<f64>,
    pub timestamp: i64,
}

impl PolygonClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn get_details(&self, ticker: &str, date: NaiveDate) -> Result<Details> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        let url = format!(
            "https://api.polygon.io/v3/reference/tickers/{}?date={}",
            ticker,
            date.format("%Y-%m-%d")
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let text = response.text().await.context("Failed to get response text")?;

        if !status.is_success() {
            anyhow::bail!("API error: {} - {}", status, text);
        }

        // Try to parse the response, if it fails, print the raw response for debugging
        match serde_json::from_str::<PolygonResponse>(&text) {
            Ok(polygon_response) => Ok(polygon_response.results),
            Err(e) => {
                eprintln!("Failed to parse response: {}", e);
                eprintln!("Raw response: {}", text);
                Err(e).context("Failed to parse response")
            }
        }
    }
}

pub async fn get_details_eu(ticker: &str, rate_map: &HashMap<String, f64>) -> Result<Details> {
    let api_key = env::var("FINANCIALMODELINGPREP_API_KEY").expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let client = FMPClient::new(api_key);
    client.get_details(ticker, rate_map).await
}
