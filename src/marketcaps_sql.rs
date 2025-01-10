use crate::api;
use crate::config;
use crate::currencies::convert_currency;
use anyhow::Result;
use chrono::Local;
use futures::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;

pub async fn marketcaps() -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

    // First fetch exchange rates
    let api_key = std::env::var("FINANCIALMODELINGPREP_API_KEY")
        .expect("FINANCIALMODELINGPREP_API_KEY must be set");
    let fmp_client = Arc::new(api::FMPClient::new(api_key));

    println!("Fetching current exchange rates...");
    let exchange_rates = match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            println!("✅ Exchange rates fetched");
            rates
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to fetch exchange rates: {}", e));
        }
    };

    // Create a map of currency pairs to rates
    let mut rate_map = std::collections::HashMap::new();
    for rate in exchange_rates {
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            rate_map.insert(name, price);
        }
    }

    // Get timestamp for data freshness tracking
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();

    // Prepare progress bar
    let pb = ProgressBar::new(tickers.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Process all tickers
    let mut results = Vec::new();
    for ticker in tickers {
        if let Ok(details) = fmp_client.get_details(&ticker, &rate_map).await {
            let market_cap_eur = details.market_cap.unwrap_or_default();
            let market_cap_usd = convert_currency(market_cap_eur, "EUR/USD", &rate_map);

            let record = vec![
                details.ticker,
                details.name.unwrap_or_default(),
                details.market_cap.map(|v| v.to_string()).unwrap_or_default(),
                details.currency_symbol.unwrap_or_default(),
                market_cap_eur.to_string(),
                market_cap_usd.to_string(),
                details.active.map(|v| v.to_string()).unwrap_or_default(),
                details.description.unwrap_or_default(),
                details.homepage_url.unwrap_or_default(),
                details.employees.unwrap_or_default(),
                details.revenue.map(|v| v.to_string()).unwrap_or_default(),
                details.revenue_usd.map(|v| v.to_string()).unwrap_or_default(),
                details.working_capital_ratio.map(|v| v.to_string()).unwrap_or_default(),
                details.quick_ratio.map(|v| v.to_string()).unwrap_or_default(),
                details.eps.map(|v| v.to_string()).unwrap_or_default(),
                details.pe_ratio.map(|v| v.to_string()).unwrap_or_default(),
                details.debt_equity_ratio.map(|v| v.to_string()).unwrap_or_default(),
                details.roe.map(|v| v.to_string()).unwrap_or_default(),
                timestamp.clone(),
            ];
            results.push((market_cap_eur, record));
        }
        pb.inc(1);
    }
    pb.finish_with_message("✅ All tickers processed");

    // Sort by market cap (descending)
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    Ok(())
}
