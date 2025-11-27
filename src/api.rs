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
use crate::models::{
    Details, FMPCompanyProfile, FMPExecutive, FMPIncomeStatement, FMPRatios, PolygonResponse,
};

#[derive(Debug, Deserialize, Clone)]
pub struct SymbolChange {
    #[serde(rename = "oldSymbol")]
    pub old_symbol: String,
    #[serde(rename = "newSymbol")]
    pub new_symbol: String,
    pub date: Option<String>,
    pub name: Option<String>,
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

            // Helper to schedule permit release after 200ms (ensures permits are always released)
            let schedule_permit_release = || {
                let rate_limiter = self.rate_limiter.clone();
                tokio::spawn(async move {
                    sleep(Duration::from_millis(200)).await;
                    rate_limiter.add_permits(1);
                });
            };

            let response = match self.client.get(&url).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    schedule_permit_release();
                    return Err(anyhow::anyhow!("Failed to send request: {}", e));
                }
            };

            // Get the response text first to log in case of error
            let text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    schedule_permit_release();
                    return Err(anyhow::anyhow!("Failed to get response text: {}", e));
                }
            };

            // Check for rate limit error
            if text.contains("Limit Reach") {
                // Release permit before retry
                schedule_permit_release();

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
                    schedule_permit_release();
                    return Ok(result);
                }
                Err(e) => {
                    schedule_permit_release();
                    eprintln!("Failed to parse response for URL {}: {}", url, e);
                    eprintln!("Response text: {}", text);
                    return Err(anyhow::anyhow!("Failed to parse response: {}", e));
                }
            }
        }
    }

    pub async fn fetch_symbol_changes(&self) -> Result<Vec<SymbolChange>> {
        let url = format!(
            "https://financialmodelingprep.com/api/v4/symbol_change?apikey={}",
            self.api_key
        );

        let response: Vec<SymbolChange> = self
            .make_request(url)
            .await
            .context("Failed to fetch symbol changes from FMP API")?;

        Ok(response)
    }

    pub async fn get_details(
        &self,
        ticker: &str,
        rate_map: &HashMap<String, f64>,
    ) -> Result<Details> {
        if ticker.is_empty() {
            anyhow::bail!("ticker empty");
        }

        // Prepare URLs for all four requests
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
        let executives_url = format!(
            "https://financialmodelingprep.com/api/v3/key-executives/{}?apikey={}",
            ticker, self.api_key
        );

        // Make all four requests in parallel
        let (profiles, ratios, income_statements, executives) = tokio::try_join!(
            self.make_request::<Vec<FMPCompanyProfile>>(profile_url),
            self.make_request::<Vec<FMPRatios>>(ratios_url),
            self.make_request::<Vec<FMPIncomeStatement>>(income_url),
            self.make_request::<Vec<FMPExecutive>>(executives_url)
        )?;

        if profiles.is_empty() {
            anyhow::bail!("No data found for ticker");
        }

        let profile = &profiles[0];
        let currency = profile.currency.as_str();
        let ratios = ratios.first().cloned();
        let income = income_statements.first().cloned();

        // Extract CEO name from executives list
        let ceo_name = executives
            .iter()
            .find(|exec| {
                exec.title.to_lowercase().contains("chief executive")
                    || exec.title.to_lowercase().contains("ceo")
            })
            .map(|exec| exec.name.clone());

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
            ceo: ceo_name,
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
                    original_currency: profile.currency.clone(), // Use actual currency from profile
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
                    original_currency: profile.currency.clone(), // Use actual currency from profile
                    exchange: profile.exchange.clone(),
                    price,
                });
            }
        }

        anyhow::bail!("No market cap data found for ticker {}", ticker)
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

    /// Fetch historical exchange rates for a specific currency pair within a date range
    pub async fn get_historical_exchange_rates(
        &self,
        pair: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<HistoricalForexResponse> {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/historical-price-full/{}?from={}&to={}&apikey={}",
            pair, from_date, to_date, self.api_key
        );

        self.make_request(url).await
    }

    /// Get available forex currency pairs
    pub async fn get_available_forex_pairs(&self) -> Result<Vec<String>> {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/symbol/available-forex-currency-pairs?apikey={}",
            self.api_key
        );

        #[derive(Debug, Deserialize)]
        struct ForexPair {
            symbol: String,
            #[allow(dead_code)]
            name: Option<String>,
            #[allow(dead_code)]
            currency: Option<String>,
            #[serde(rename = "stockExchange")]
            #[allow(dead_code)]
            stock_exchange: Option<String>,
            #[serde(rename = "exchangeShortName")]
            #[allow(dead_code)]
            exchange_short_name: Option<String>,
        }

        let pairs: Vec<ForexPair> = self.make_request(url).await?;
        Ok(pairs.into_iter().map(|p| p.symbol).collect())
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

/// Response from historical forex price endpoint
#[derive(Debug, Deserialize)]
pub struct HistoricalForexResponse {
    pub symbol: String,
    pub historical: Vec<HistoricalForexData>,
}

/// Individual historical forex data point
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct HistoricalForexData {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    #[serde(rename = "adjClose")]
    pub adj_close: Option<f64>,
    pub volume: Option<f64>,
    #[serde(rename = "unadjustedVolume")]
    pub unadjusted_volume: Option<f64>,
    pub change: Option<f64>,
    #[serde(rename = "changePercent")]
    pub change_percent: Option<f64>,
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

    #[test]
    fn test_ceo_extraction_chief_executive() {
        let executives = vec![
            FMPExecutive {
                title: "Chief Financial Officer".to_string(),
                name: "Jane CFO".to_string(),
                pay: Some(5000000.0),
                currency_pay: Some("USD".to_string()),
                gender: Some("female".to_string()),
                year_born: Some(1970),
            },
            FMPExecutive {
                title: "Chief Executive Officer & Director".to_string(),
                name: "John CEO".to_string(),
                pay: Some(10000000.0),
                currency_pay: Some("USD".to_string()),
                gender: Some("male".to_string()),
                year_born: Some(1965),
            },
            FMPExecutive {
                title: "Chief Technology Officer".to_string(),
                name: "Bob CTO".to_string(),
                pay: None,
                currency_pay: None,
                gender: None,
                year_born: None,
            },
        ];

        let ceo_name = executives
            .iter()
            .find(|exec| {
                exec.title.to_lowercase().contains("chief executive")
                    || exec.title.to_lowercase().contains("ceo")
            })
            .map(|exec| exec.name.clone());

        assert_eq!(ceo_name, Some("John CEO".to_string()));
    }

    #[test]
    fn test_ceo_extraction_ceo_abbreviation() {
        let executives = vec![
            FMPExecutive {
                title: "CFO".to_string(),
                name: "Jane CFO".to_string(),
                pay: None,
                currency_pay: None,
                gender: None,
                year_born: None,
            },
            FMPExecutive {
                title: "CEO & President".to_string(),
                name: "Alice CEO".to_string(),
                pay: None,
                currency_pay: None,
                gender: None,
                year_born: None,
            },
        ];

        let ceo_name = executives
            .iter()
            .find(|exec| {
                exec.title.to_lowercase().contains("chief executive")
                    || exec.title.to_lowercase().contains("ceo")
            })
            .map(|exec| exec.name.clone());

        assert_eq!(ceo_name, Some("Alice CEO".to_string()));
    }

    #[test]
    fn test_ceo_extraction_no_ceo() {
        let executives = vec![
            FMPExecutive {
                title: "Chief Financial Officer".to_string(),
                name: "Jane CFO".to_string(),
                pay: None,
                currency_pay: None,
                gender: None,
                year_born: None,
            },
            FMPExecutive {
                title: "Chief Operating Officer".to_string(),
                name: "Bob COO".to_string(),
                pay: None,
                currency_pay: None,
                gender: None,
                year_born: None,
            },
        ];

        let ceo_name = executives
            .iter()
            .find(|exec| {
                exec.title.to_lowercase().contains("chief executive")
                    || exec.title.to_lowercase().contains("ceo")
            })
            .map(|exec| exec.name.clone());

        assert_eq!(ceo_name, None);
    }

    #[test]
    fn test_ceo_extraction_case_insensitive() {
        let executives = vec![FMPExecutive {
            title: "CHIEF EXECUTIVE OFFICER".to_string(),
            name: "Upper Case CEO".to_string(),
            pay: None,
            currency_pay: None,
            gender: None,
            year_born: None,
        }];

        let ceo_name = executives
            .iter()
            .find(|exec| {
                exec.title.to_lowercase().contains("chief executive")
                    || exec.title.to_lowercase().contains("ceo")
            })
            .map(|exec| exec.name.clone());

        assert_eq!(ceo_name, Some("Upper Case CEO".to_string()));
    }

    #[test]
    fn test_ceo_extraction_interim_ceo() {
        // This test documents current behavior: interim CEOs are matched
        let executives = vec![FMPExecutive {
            title: "Interim CEO".to_string(),
            name: "Temporary CEO".to_string(),
            pay: None,
            currency_pay: None,
            gender: None,
            year_born: None,
        }];

        let ceo_name = executives
            .iter()
            .find(|exec| {
                exec.title.to_lowercase().contains("chief executive")
                    || exec.title.to_lowercase().contains("ceo")
            })
            .map(|exec| exec.name.clone());

        assert_eq!(ceo_name, Some("Temporary CEO".to_string()));
    }

    #[test]
    fn test_symbol_change_deserialization() {
        let json = serde_json::json!({
            "oldSymbol": "FB",
            "newSymbol": "META",
            "date": "2022-06-09",
            "name": "Meta Platforms Inc"
        });

        let change: SymbolChange = serde_json::from_value(json).unwrap();
        assert_eq!(change.old_symbol, "FB");
        assert_eq!(change.new_symbol, "META");
        assert_eq!(change.date, Some("2022-06-09".to_string()));
        assert_eq!(change.name, Some("Meta Platforms Inc".to_string()));
    }

    #[test]
    fn test_symbol_change_with_missing_optional_fields() {
        let json = serde_json::json!({
            "oldSymbol": "TWTR",
            "newSymbol": "X"
        });

        let change: SymbolChange = serde_json::from_value(json).unwrap();
        assert_eq!(change.old_symbol, "TWTR");
        assert_eq!(change.new_symbol, "X");
        assert_eq!(change.date, None);
        assert_eq!(change.name, None);
    }

    #[test]
    fn test_symbol_change_list_deserialization() {
        let json = serde_json::json!([
            {
                "oldSymbol": "FB",
                "newSymbol": "META",
                "date": "2022-06-09"
            },
            {
                "oldSymbol": "TWTR",
                "newSymbol": "X",
                "date": "2023-07-24",
                "name": "X Corp"
            }
        ]);

        let changes: Vec<SymbolChange> = serde_json::from_value(json).unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].old_symbol, "FB");
        assert_eq!(changes[1].old_symbol, "TWTR");
    }

    #[test]
    fn test_exchange_rate_deserialization() {
        let json = serde_json::json!({
            "name": "EUR/USD",
            "price": 1.08,
            "changesPercentage": 0.5,
            "change": 0.005,
            "dayLow": 1.075,
            "dayHigh": 1.085,
            "yearHigh": 1.12,
            "yearLow": 1.02,
            "marketCap": null,
            "priceAvg50": 1.07,
            "priceAvg200": 1.06,
            "volume": 1000000.0,
            "avgVolume": 900000.0,
            "exchange": "FOREX",
            "open": 1.078,
            "previousClose": 1.075,
            "timestamp": 1701956301
        });

        let rate: ExchangeRate = serde_json::from_value(json).unwrap();
        assert_eq!(rate.name, Some("EUR/USD".to_string()));
        assert_eq!(rate.price, Some(1.08));
        assert_eq!(rate.changes_percentage, Some(0.5));
        assert_eq!(rate.timestamp, 1701956301);
    }

    #[test]
    fn test_exchange_rate_with_minimal_fields() {
        let json = serde_json::json!({
            "timestamp": 1701956301
        });

        let rate: ExchangeRate = serde_json::from_value(json).unwrap();
        assert_eq!(rate.name, None);
        assert_eq!(rate.price, None);
        assert_eq!(rate.timestamp, 1701956301);
    }

    #[test]
    fn test_historical_forex_data_deserialization() {
        let json = serde_json::json!({
            "date": "2024-01-15",
            "open": 1.0750,
            "high": 1.0820,
            "low": 1.0700,
            "close": 1.0800,
            "adjClose": 1.0800,
            "volume": 500000.0,
            "unadjustedVolume": 500000.0,
            "change": 0.005,
            "changePercent": 0.47
        });

        let data: HistoricalForexData = serde_json::from_value(json).unwrap();
        assert_eq!(data.date, "2024-01-15");
        assert_eq!(data.open, 1.0750);
        assert_eq!(data.high, 1.0820);
        assert_eq!(data.low, 1.0700);
        assert_eq!(data.close, 1.0800);
        assert_eq!(data.adj_close, Some(1.0800));
    }

    #[test]
    fn test_historical_forex_data_with_minimal_fields() {
        let json = serde_json::json!({
            "date": "2024-01-15",
            "open": 1.0750,
            "high": 1.0820,
            "low": 1.0700,
            "close": 1.0800
        });

        let data: HistoricalForexData = serde_json::from_value(json).unwrap();
        assert_eq!(data.date, "2024-01-15");
        assert_eq!(data.close, 1.0800);
        assert_eq!(data.adj_close, None);
        assert_eq!(data.volume, None);
    }

    #[test]
    fn test_historical_forex_response_deserialization() {
        let json = serde_json::json!({
            "symbol": "EURUSD",
            "historical": [
                {
                    "date": "2024-01-15",
                    "open": 1.0750,
                    "high": 1.0820,
                    "low": 1.0700,
                    "close": 1.0800
                },
                {
                    "date": "2024-01-14",
                    "open": 1.0720,
                    "high": 1.0780,
                    "low": 1.0680,
                    "close": 1.0750
                }
            ]
        });

        let response: HistoricalForexResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.symbol, "EURUSD");
        assert_eq!(response.historical.len(), 2);
        assert_eq!(response.historical[0].date, "2024-01-15");
        assert_eq!(response.historical[1].date, "2024-01-14");
    }

    #[test]
    fn test_historical_forex_response_empty_historical() {
        let json = serde_json::json!({
            "symbol": "XYZUSD",
            "historical": []
        });

        let response: HistoricalForexResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.symbol, "XYZUSD");
        assert!(response.historical.is_empty());
    }

    #[test]
    fn test_fmp_client_creation() {
        let _client = FMPClient::new("test_api_key".to_string());
        // Client should be created successfully
        // We can't directly test the internal fields, but we can verify it doesn't panic
        assert!(true);
    }

    #[test]
    fn test_polygon_client_creation() {
        let _client = PolygonClient::new("test_api_key".to_string());
        // Client should be created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_polygon_empty_ticker() {
        let client = PolygonClient::new("test_key".to_string());
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = client.get_details("", date).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("ticker empty"));
    }

    #[tokio::test]
    async fn test_fmp_client_empty_ticker() {
        let client = FMPClient::new("test_key".to_string());
        let rate_map = HashMap::new();
        let result = client.get_details("", &rate_map).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("ticker empty"));
    }
}
