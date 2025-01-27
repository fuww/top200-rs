// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::{env, time::Duration};
use tokio::sync::Semaphore;
use tokio::time::sleep;

use crate::currencies::convert_currency;
use crate::models::{Details, FMPCompanyProfile, FMPIncomeStatement, FMPRatios, PolygonResponse};

use mockall::automock;

#[automock]
pub trait FMPClientTrait {
    async fn get_details(&self, ticker: &str, rate_map: &HashMap<String, f64>) -> Result<Details>;
    async fn get_historical_market_cap(&self, ticker: &str, date: &DateTime<Utc>) -> Result<HistoricalMarketCap>;
    async fn get_ratios(&self, ticker: &str) -> Result<Option<FMPRatios>>;
    async fn get_income_statement(&self, ticker: &str) -> Result<Option<FMPIncomeStatement>>;
    async fn get_exchange_rates(&self) -> Result<Vec<ExchangeRate>>;
}

pub struct PolygonClient {
    client: Client,
    api_key: String,
}

#[derive(Clone)]
pub struct FMPClient {
    client: Client,
    api_key: String,
    rate_limiter: Arc<Semaphore>,
}

#[async_trait::async_trait]
impl FMPClientTrait for FMPClient {
    async fn get_details(&self, ticker: &str, rate_map: &HashMap<String, f64>) -> Result<Details> {
        self.get_details(ticker, rate_map).await
    }

    async fn get_historical_market_cap(&self, ticker: &str, date: &DateTime<Utc>) -> Result<HistoricalMarketCap> {
        self.get_historical_market_cap(ticker, date).await
    }

    async fn get_ratios(&self, ticker: &str) -> Result<Option<FMPRatios>> {
        self.get_ratios(ticker).await
    }

    async fn get_income_statement(&self, ticker: &str) -> Result<Option<FMPIncomeStatement>> {
        self.get_income_statement(ticker).await
    }

    async fn get_exchange_rates(&self) -> Result<Vec<ExchangeRate>> {
        self.get_exchange_rates().await
    }
}

impl FMPClient {
    pub fn new(api_key: String) -> Self {
        // Allow up to 300 concurrent requests per minute
        let rate_limiter = Arc::new(Semaphore::new(300));

        Self {
            client: Client::new(),
            api_key,
            rate_limiter,
        }
    }

    async fn make_request<T: for<'de> Deserialize<'de>>(&self, url: String) -> Result<T> {
        let mut retries = 0;
        let max_retries = 3;
        let mut delay = Duration::from_secs(5);

        loop {
            // Wait for rate limit permit
            let _permit = self.rate_limiter.acquire().await.unwrap();

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to send request")?;

            // Get the response text first to log in case of error
            let text = response
                .text()
                .await
                .context("Failed to get response text")?;

            // Check for rate limit error
            if text.contains("Limit Reach") {
                if retries >= max_retries {
                    return Err(anyhow::anyhow!(
                        "Rate limit reached after {} retries",
                        max_retries
                    ));
                }
                eprintln!(
                    "Rate limit hit for {}. Retrying in {} seconds...",
                    url,
                    delay.as_secs()
                );
                sleep(delay).await;
                delay *= 2; // Exponential backoff
                retries += 1;
                continue;
            }

            match serde_json::from_str::<T>(&text) {
                Ok(result) => {
                    // Schedule permit release after 200ms
                    let rate_limiter = self.rate_limiter.clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_millis(200)).await;
                        rate_limiter.add_permits(1);
                    });
                    return Ok(result);
                }
                Err(e) => {
                    eprintln!("Failed to parse response for URL {}: {}", url, e);
                    eprintln!("Response text: {}", text);
                    return Err(anyhow::anyhow!("Failed to parse response: {}", e));
                }
            }
        }
    }

    pub async fn get_details(
        &self,
        ticker: &str,
        rate_map: &HashMap<String, f64>,
    ) -> Result<Details> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        // Prepare URLs for all three requests
        let profile_url = format!(
            "https://financialmodelingprep.com/api/v3/profile/{}?apikey={}",
            ticker, self.api_key
        );
        let ratios_url = format!(
            "https://financialmodelingprep.com/api/v3/ratios/{}?apikey={}",
            ticker, self.api_key
        );
        let income_url = format!(
            "https://financialmodelingprep.com/api/v3/income-statement/{}?limit=1&apikey={}",
            ticker, self.api_key
        );

        // Make all three requests in parallel
        let (profiles, ratios, income_statements) = tokio::try_join!(
            self.make_request::<Vec<FMPCompanyProfile>>(profile_url),
            self.make_request::<Vec<FMPRatios>>(ratios_url),
            self.make_request::<Vec<FMPIncomeStatement>>(income_url)
        )?;

        if profiles.is_empty() {
            anyhow::bail!("No data found for ticker");
        }

        let profile = &profiles[0];
        let currency = profile.currency.as_str();
        let ratios = ratios.first().cloned();
        let income = income_statements.first().cloned();

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
            weighted_shares_outstanding: None,
            employees: profile.employees.clone(),
            revenue: income.as_ref().and_then(|i| i.revenue),
            revenue_usd: None,
            timestamp: Some(timestamp),
            working_capital_ratio: ratios.as_ref().and_then(|r| r.current_ratio),
            quick_ratio: ratios.as_ref().and_then(|r| r.quick_ratio),
            eps: ratios.as_ref().and_then(|r| r.eps),
            pe_ratio: ratios.as_ref().and_then(|r| r.price_earnings_ratio),
            debt_equity_ratio: ratios.as_ref().and_then(|r| r.debt_equity_ratio),
            roe: ratios.as_ref().and_then(|r| r.return_on_equity),
            extra: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "exchange".to_string(),
                    Value::String(profile.exchange.clone()),
                );
                map.insert(
                    "price".to_string(),
                    Value::Number(
                        serde_json::Number::from_f64(profile.price)
                            .unwrap_or(serde_json::Number::from(0)),
                    ),
                );
                map
            },
        };

        // Calculate revenue in USD if available
        if let Some(rev) = details.revenue {
            details.revenue_usd = Some(convert_currency(rev, currency, "USD", rate_map));
        }

        Ok(details)
    }

    pub async fn get_historical_market_cap(
        &self,
        ticker: &str,
        date: &DateTime<Utc>,
    ) -> Result<HistoricalMarketCap> {
        // First try historical market cap endpoint
        let url = format!(
            "https://financialmodelingprep.com/api/v3/historical-market-capitalization/{}?from={}&to={}&apikey={}",
            ticker,
            date.format("%Y-%m-%d"),
            date.format("%Y-%m-%d"),
            self.api_key
        );

        let response: Vec<Value> = self.make_request(url).await?;

        if let Some(data) = response.first() {
            let market_cap = data["marketCap"].as_f64().unwrap_or(0.0);
            let price = data["price"].as_f64().unwrap_or(0.0);

            // Get company profile for additional info
            let profile_url = format!(
                "https://financialmodelingprep.com/api/v3/profile/{}?apikey={}",
                ticker, self.api_key
            );
            let profiles: Vec<FMPCompanyProfile> = self.make_request(profile_url).await?;

            if let Some(profile) = profiles.first() {
                return Ok(HistoricalMarketCap {
                    ticker: ticker.to_string(),
                    name: profile.company_name.clone(),
                    market_cap_original: market_cap,
                    original_currency: "USD".to_string(), // FMP returns USD by default
                    exchange: profile.exchange.clone(),
                    price,
                });
            }
        }

        // If historical data not found, try the quote endpoint
        let quote_url = format!(
            "https://financialmodelingprep.com/api/v3/quote/{}?apikey={}",
            ticker, self.api_key
        );

        let quotes: Vec<Value> = self.make_request(quote_url).await?;

        if let Some(quote) = quotes.first() {
            let market_cap = quote["marketCap"].as_f64().unwrap_or(0.0);
            let price = quote["price"].as_f64().unwrap_or(0.0);

            // Get company profile for additional info
            let profile_url = format!(
                "https://financialmodelingprep.com/api/v3/profile/{}?apikey={}",
                ticker, self.api_key
            );
            let profiles: Vec<FMPCompanyProfile> = self.make_request(profile_url).await?;

            if let Some(profile) = profiles.first() {
                return Ok(HistoricalMarketCap {
                    ticker: ticker.to_string(),
                    name: profile.company_name.clone(),
                    market_cap_original: market_cap,
                    original_currency: "USD".to_string(), // FMP returns USD by default
                    exchange: profile.exchange.clone(),
                    price,
                });
            }
        }

        anyhow::bail!("No market cap data found for ticker {}", ticker)
    }

    pub async fn get_ratios(&self, ticker: &str) -> Result<Option<FMPRatios>> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        let url = format!(
            "https://financialmodelingprep.com/api/v3/ratios/{}?apikey={}",
            ticker, self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let text = response
            .text()
            .await
            .context("Failed to get response text")?;

        if !status.is_success() {
            anyhow::bail!("API request failed: {}", text);
        }

        let ratios: Vec<FMPRatios> =
            serde_json::from_str(&text).context("Failed to parse FMP ratios response")?;

        // Get the most recent ratios (first in the list)
        Ok(ratios.into_iter().next())
    }

    pub async fn get_income_statement(&self, ticker: &str) -> Result<Option<FMPIncomeStatement>> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        let url = format!(
            "https://financialmodelingprep.com/api/v3/income-statement/{}?limit=1&apikey={}",
            ticker, self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let text = response
            .text()
            .await
            .context("Failed to get response text")?;

        if !status.is_success() {
            anyhow::bail!("API request failed: {}", text);
        }

        let statements: Vec<FMPIncomeStatement> =
            serde_json::from_str(&text).context("Failed to parse FMP income statement response")?;

        // Get the most recent statement (first in the list)
        Ok(statements.into_iter().next())
    }

    pub async fn get_exchange_rates(&self) -> Result<Vec<ExchangeRate>> {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/quotes/forex?apikey={}",
            self.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to FMP forex API")?;

        if !response.status().is_success() {
            anyhow::bail!("API request failed with status: {}", response.status());
        }

        let rates: Vec<ExchangeRate> = response
            .json()
            .await
            .context("Failed to parse forex rates response")?;
        Ok(rates)
    }
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
        let text = response
            .text()
            .await
            .context("Failed to get response text")?;

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
    let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let client = FMPClient::new(api_key);
    client.get_details(ticker, rate_map).await
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ExchangeRate {
    pub name: Option<String>,
    pub price: Option<f64>,
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

#[derive(Debug, Deserialize)]
pub struct HistoricalMarketCap {
    #[allow(dead_code)]
    pub ticker: String,
    pub name: String,
    pub market_cap_original: f64,
    pub original_currency: String,
    pub exchange: String,
    pub price: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_empty_ticker() {
        let client = FMPClient::new("test_key".to_string());
        let rate_map = HashMap::new();
        let result = client.get_details("", &rate_map).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ticker empty"));

        let client = PolygonClient::new("test_key".to_string());
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = client.get_details("", date).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ticker empty"));
    }
}
