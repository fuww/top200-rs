mod api;
mod models;
mod viz;

use anyhow::Result;
use chrono::{Local, NaiveDate};
use csv::Writer;
use dotenv::dotenv;
use inquire::Select;
use std::{env, path::PathBuf, time::Duration};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // Read the latest CSV file
    let csv_path = "output/combined_marketcaps_20241205_171822.csv";
    let mut rdr = csv::Reader::from_path(csv_path)?;
    
    let mut stocks = Vec::new();
    for result in rdr.records() {
        let record = result?;
        stocks.push(viz::StockData {
            symbol: record[0].to_string(),
            market_cap_eur: record[4].parse::<f64>().unwrap_or(0.0),
            price_change: record[7].parse::<f64>().unwrap_or(0.0),
        });
    }

    let viz_output_path = format!("output/market_heatmap_{}.png", 
        Local::now().format("%Y%m%d_%H%M%S"));
    viz::create_market_heatmap(stocks, &viz_output_path)?;
    
    println!("Market heatmap saved to: {}", viz_output_path);
    Ok(())
}

async fn export_details_eu_csv() -> Result<()> {
    let tickers = vec![
        "ASML", "LVMH.PA", "NOVO-B.CO", "ROG.SW", "MC.PA", 
        "SAP.DE", "BALN.SW", "SAN.PA", "AIR.PA", "OR.PA"
    ];

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("eu_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Ticker",
        "Company Name",
        "Market Cap",
        "Currency",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
    ])?;

    for (i, ticker) in tickers.iter().enumerate() {
        println!("\nFetching the marketcap for {} ({}/{}) ⌛️", ticker, i + 1, tickers.len());
        match api::get_details_eu(ticker).await {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details.market_cap.map(|m| m.to_string()).unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.extra.get("exchange").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    &details.extra.get("price").map(|v| v.to_string()).unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    ticker,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    &error_msg,
                    "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", csv_path.display());

    Ok(())
}

async fn export_details_us_csv() -> Result<()> {
    let api_key = env::var("POLYGON_API_KEY").expect("POLYGON_API_KEY must be set");
    let client = api::PolygonClient::new(api_key);
    let date = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();

    let tickers = vec![
        "NKE", "TJX", "VFC", "GPS", "PVH", "M", "RL", "JWN", "HBI", "UA",
        "CRI", "FL", "COLM", "LULU", "URBN", "AEO", "DKS",
    ];

    // Create output directory if it doesn't exist
    let output_dir = PathBuf::from("output");
    std::fs::create_dir_all(&output_dir)?;

    // Create CSV file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let csv_path = output_dir.join(format!("us_marketcaps_{}.csv", timestamp));
    let mut writer = Writer::from_path(&csv_path)?;

    // Write header
    writer.write_record(&[
        "Ticker",
        "Company Name",
        "Market Cap",
        "Currency",
        "Active",
        "Description",
        "Homepage URL",
    ])?;

    for (i, ticker) in tickers.iter().enumerate() {
        println!("\nFetching the marketcap for {} ({}/{}) ⌛️", ticker, i + 1, tickers.len());
        match client.get_details(ticker, date).await {
            Ok(details) => {
                writer.write_record(&[
                    &details.ticker,
                    &details.name.unwrap_or_default(),
                    &details.market_cap.map(|m| m.to_string()).unwrap_or_default(),
                    &details.currency_symbol.unwrap_or_default(),
                    &details.active.map(|a| a.to_string()).unwrap_or_default(),
                    &details.description.unwrap_or_default(),
                    &details.homepage_url.unwrap_or_default(),
                ])?;
                println!("✅ Data written to CSV");
            }
            Err(e) => {
                eprintln!("Error fetching details for {}: {}", ticker, e);
                // Write empty row for failed ticker
                let error_msg = format!("Error: {}", e);
                writer.write_record(&[
                    ticker,
                    "",
                    "",
                    "",
                    "",
                    &error_msg,
                    "",
                ])?;
            }
        }
    }

    writer.flush()?;
    println!("\n✅ CSV file created at: {}", csv_path.display());

    Ok(())
}

async fn list_details_us() -> Result<()> {
    let api_key = env::var("POLYGON_API_KEY").expect("POLYGON_API_KEY must be set");
    let client = api::PolygonClient::new(api_key);
    let date = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();

    let tickers = vec![
        "NKE", "TJX", "VFC", "GPS", "PVH", "M", "RL", "JWN", "HBI", "UA",
        "CRI", "FL", "COLM", "LULU", "URBN", "AEO", "DKS",
    ];

    for (i, ticker) in tickers.iter().enumerate() {
        println!("\nFetching the marketcap for {} ({}/{}) ⌛️", ticker, i + 1, tickers.len());
        match client.get_details(ticker, date).await {
            Ok(details) => {
                println!("Company: {}", details.name.unwrap_or_default());
                if let Some(market_cap) = details.market_cap {
                    println!("Market Cap: {} {}", details.currency_symbol.unwrap_or_default(), market_cap);
                }
                println!("Active: {}", details.active.unwrap_or_default());
                println!("---");
            }
            Err(e) => eprintln!("Error fetching details for {}: {}", ticker, e),
        }
    }

    Ok(())
}

async fn list_details_eu() -> Result<()> {
    let tickers = vec![
        "ASML", "LVMH.PA", "NOVO-B.CO", "ROG.SW", "MC.PA", 
        "SAP.DE", "BALN.SW", "SAN.PA", "AIR.PA", "OR.PA"
    ];

    for (i, ticker) in tickers.iter().enumerate() {
        println!("\nFetching the marketcap for {} ({}/{}) ⌛️", ticker, i + 1, tickers.len());
        match api::get_details_eu(ticker).await {
            Ok(details) => {
                println!("Company: {}", details.name.unwrap_or_default());
                if let Some(market_cap) = details.market_cap {
                    println!("Market Cap: {} {}", details.currency_symbol.unwrap_or_default(), market_cap);
                }
                println!("Active: {}", details.active.unwrap_or_default());
                println!("---");
            }
            Err(e) => eprintln!("Error fetching details for {}: {}", ticker, e),
        }
    }

    Ok(())
}

async fn export_details_combined_csv(fmp_client: &api::FMPClient) -> Result<()> {
    // First fetch exchange rates
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
    let mut rate_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for rate in exchange_rates {
        rate_map.insert(rate.name.clone(), rate.price);
    }

    // Helper function to convert currency symbol to code and get divisor
    let currency_to_code = |currency: &str| -> (String, f64) {
        match currency {
            "€" => ("EUR".to_string(), 1.0),
            "$" => ("USD".to_string(), 1.0),
            "£" => ("GBP".to_string(), 1.0),
            "¥" => ("JPY".to_string(), 1.0),
            "₣" => ("CHF".to_string(), 1.0),
            "kr" => ("SEK".to_string(), 1.0),
            "GBp" => ("GBP".to_string(), 100.0),  // Convert pence to pounds
            "GBX" => ("GBP".to_string(), 100.0),  // Alternative notation for pence
            _ => (currency.to_string(), 1.0)
        }
    };

    // Helper function to convert amount from source currency to target currency
    let convert_currency = |amount: f64, from_currency: &str, to_currency: &str| -> f64 {
        let (from_code, from_divisor) = currency_to_code(from_currency);
        let amount = amount / from_divisor; // Convert to main currency unit if needed
        
        if from_code == to_currency {
            amount
        } else {
            // First try direct conversion
            let pair = format!("{}/{}", from_code, to_currency);
            if let Some(&rate) = rate_map.get(&pair) {
                amount * rate
            } else {
                // Try inverse conversion
                let inverse_pair = format!("{}/{}", to_currency, from_code);
                if let Some(&rate) = rate_map.get(&inverse_pair) {
                    amount / rate
                } else {
                    // If no direct conversion exists, try through USD
                    let to_usd = format!("{}/USD", from_code);
                    let usd_to_target = rate_map.get(&format!("USD/{}", to_currency)).copied().unwrap_or_else(|| {
                        if to_currency == "EUR" {
                            0.92 // Fallback USD to EUR
                        } else {
                            1.0 // Fallback for USD
                        }
                    });
                    
                    if let Some(&rate) = rate_map.get(&to_usd) {
                        amount * rate * usd_to_target
                    } else {
                        println!("⚠️  Warning: No conversion rate found for {} to {}", from_currency, to_currency);
                        amount // Return unconverted amount as fallback
                    }
                }
            }
        }
    };

    // Convert exchange prefixes to FMP format
    let tickers = vec![
        "MC.PA",     // LVMH (Paris)
        "NKE",       // Nike (NYSE)
        "ITX.MC",    // Inditex (Madrid)
        "CDI.PA",    // Dior (Paris)
        "KER.PA",    // Kering (Paris)
        "RMS.PA",    // Hermès (Paris)
        "TJX",       // TJX (NYSE)
        "CFR.SW",    // Richemont (Swiss)
        "ADS.DE",    // adidas (German)
        "9983.T",    // Fast Retailing (Tokyo)
        "EL.PA",     // EssilorLuxottica (Milan)
        "ROST",      // Ross Stores (NASDAQ)
        "VFC",       // VF Corp (NYSE)
        "HM-B.ST",   // H&M (Stockholm)
        "UHR.SW",    // Swatch Group (Swiss)
        "TPR",       // Coach/Tapestry (NYSE) - COH is now TPR
        "ZAL.DE",    // Zalando (German)
        "GPS",       // Gap (NYSE)
        "BBWI",      // L Brands is now Bath & Body Works (NYSE)
        "BRBY.L",    // Burberry (London)
        "1929.HK",   // Chow Tai Fook (Hong Kong)
        "PVH",       // PVH (NYSE)
        "LULU",      // Lululemon (NASDAQ)
        "1913.HK",   // Prada Group (Hong Kong)
        "VIPS",      // Vipshop Holdings (NYSE)
        "NXT.L",     // Next (London)
        "CPRI",      // Michael Kors is now Capri Holdings (NYSE)
        "M",         // Macy's (NYSE)
        "MONC.MI",   // Moncler (Milan)
        "RL",        // Ralph Lauren (NYSE)
        "JWN",       // Nordstrom (NYSE)
        "ASC.L",     // ASOS (London)
        "LREN3.SA",  // Lojas Renner (Brazil)
        "HBI",       // Hanes (NYSE)
        "UAA",       // Under Armour (NYSE)
        "PUM.DE",    // PUMA (German)
        "MKS.L",     // Marks & Spencer (London)
        "SKX",       // Skechers (NYSE)
        "BOSS.DE",   // Hugo Boss (German)
        "GIL.TO",    // Gildan (Toronto)
    ];
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/combined_marketcaps_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = csv::Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
    ])?;

    // Collect all results first
    let mut results = Vec::new();
    for ticker in tickers {
        println!("Fetching data for {}", ticker);
        match fmp_client.get_details(ticker).await {
            Ok(details) => {
                let original_market_cap = details.market_cap.unwrap_or(0.0);
                let currency = details.currency_symbol.clone().unwrap_or_default();
                let eur_market_cap = convert_currency(original_market_cap, &currency, "EUR");
                let usd_market_cap = convert_currency(original_market_cap, &currency, "USD");
                
                results.push((
                    eur_market_cap,
                    vec![
                        details.ticker,
                        details.name.unwrap_or_default(),
                        original_market_cap.round().to_string(),
                        currency,
                        eur_market_cap.round().to_string(),
                        usd_market_cap.round().to_string(),
                        details.extra.get("exchange").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        details.extra.get("price").map(|v| v.to_string()).unwrap_or_default(),
                        details.active.map(|a| a.to_string()).unwrap_or_default(),
                        details.description.unwrap_or_default(),
                        details.homepage_url.unwrap_or_default(),
                    ]
                ));
                println!("✅ Data collected");
            }
            Err(e) => {
                eprintln!("Error fetching data for {}: {}", ticker, e);
                results.push((
                    0.0,
                    vec![
                        ticker.to_string(),
                        "ERROR".to_string(),
                        "0".to_string(),
                        "".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        format!("Error: {}", e),
                        "".to_string(),
                    ]
                ));
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Sort by EUR market cap (highest to lowest)
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Write sorted results to CSV
    for (_, record) in &results {
        writer.write_record(record)?;
    }

    // After writing the CSV, create the visualization
    let stocks: Vec<viz::StockData> = results
        .iter()
        .map(|(_, record)| viz::StockData {
            symbol: record[0].clone(),
            market_cap_eur: record[4].parse::<f64>().unwrap_or(0.0),
            price_change: record[7].parse::<f64>().unwrap_or(0.0),
        })
        .collect();

    let viz_output_path = format!("output/market_heatmap_{}.png", 
        Local::now().format("%Y%m%d_%H%M%S"));
    viz::create_market_heatmap(stocks, &viz_output_path)?;
    
    println!("Market heatmap saved to: {}", viz_output_path);
    println!("📝 CSV file created: {}", filename);
    println!("💶 Results are sorted by market cap in EUR (highest to lowest)");
    Ok(())
}

async fn export_exchange_rates_csv(fmp_client: &api::FMPClient) -> Result<()> {
    println!("Fetching current exchange rates...");
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/exchange_rates_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = csv::Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Symbol",
        "Rate",
        "Change %",
        "Change",
        "Day Low",
        "Day High",
        "Base Currency",
        "Quote Currency",
        "Timestamp",
    ])?;

    match fmp_client.get_exchange_rates().await {
        Ok(rates) => {
            for rate in rates {
                // Split the symbol into base and quote currencies (e.g., "EUR/USD" -> ["EUR", "USD"])
                let currencies: Vec<&str> = rate.name.split('/').collect();
                let (base, quote) = if currencies.len() == 2 {
                    (currencies[0], currencies[1])
                } else {
                    ("", "")
                };

                writer.write_record(&[
                    &rate.name,
                    &rate.price.to_string(),
                    &rate.changes_percentage.map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate.change.map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate.day_low.map_or_else(|| "".to_string(), |v| v.to_string()),
                    &rate.day_high.map_or_else(|| "".to_string(), |v| v.to_string()),
                    base,
                    quote,
                    &rate.timestamp.to_string(),
                ])?;
            }
            println!("✅ Exchange rates written to CSV");
        }
        Err(e) => {
            eprintln!("Error fetching exchange rates: {}", e);
            writer.write_record(&[
                "ERROR",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &format!("Error: {}", e),
            ])?;
        }
    }

    println!("📝 CSV file created: {}", filename);
    Ok(())
}

async fn export_combined_marketcaps(fmp_client: &api::FMPClient) -> Result<()> {
    // First fetch exchange rates
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
    let mut rate_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for rate in exchange_rates {
        rate_map.insert(rate.name.clone(), rate.price);
    }

    // Helper function to convert currency symbol to code and get divisor
    let currency_to_code = |currency: &str| -> (String, f64) {
        match currency {
            "€" => ("EUR".to_string(), 1.0),
            "$" => ("USD".to_string(), 1.0),
            "£" => ("GBP".to_string(), 1.0),
            "¥" => ("JPY".to_string(), 1.0),
            "₣" => ("CHF".to_string(), 1.0),
            "kr" => ("SEK".to_string(), 1.0),
            "GBp" => ("GBP".to_string(), 100.0),  // Convert pence to pounds
            "GBX" => ("GBP".to_string(), 100.0),  // Alternative notation for pence
            _ => (currency.to_string(), 1.0)
        }
    };

    // Helper function to convert amount from source currency to target currency
    let convert_currency = |amount: f64, from_currency: &str, to_currency: &str| -> f64 {
        let (from_code, from_divisor) = currency_to_code(from_currency);
        let amount = amount / from_divisor; // Convert to main currency unit if needed
        
        if from_code == to_currency {
            amount
        } else {
            // First try direct conversion
            let pair = format!("{}/{}", from_code, to_currency);
            if let Some(&rate) = rate_map.get(&pair) {
                amount * rate
            } else {
                // Try inverse conversion
                let inverse_pair = format!("{}/{}", to_currency, from_code);
                if let Some(&rate) = rate_map.get(&inverse_pair) {
                    amount / rate
                } else {
                    // If no direct conversion exists, try through USD
                    let to_usd = format!("{}/USD", from_code);
                    let usd_to_target = rate_map.get(&format!("USD/{}", to_currency)).copied().unwrap_or_else(|| {
                        if to_currency == "EUR" {
                            0.92 // Fallback USD to EUR
                        } else {
                            1.0 // Fallback for USD
                        }
                    });
                    
                    if let Some(&rate) = rate_map.get(&to_usd) {
                        amount * rate * usd_to_target
                    } else {
                        println!("⚠️  Warning: No conversion rate found for {} to {}", from_currency, to_currency);
                        amount // Return unconverted amount as fallback
                    }
                }
            }
        }
    };

    // Convert exchange prefixes to FMP format
    let tickers = vec![
        "MC.PA",     // LVMH (Paris)
        "NKE",       // Nike (NYSE)
        "ITX.MC",    // Inditex (Madrid)
        "CDI.PA",    // Dior (Paris)
        "KER.PA",    // Kering (Paris)
        "RMS.PA",    // Hermès (Paris)
        "TJX",       // TJX (NYSE)
        "CFR.SW",    // Richemont (Swiss)
        "ADS.DE",    // adidas (German)
        "9983.T",    // Fast Retailing (Tokyo)
        "EL.PA",     // EssilorLuxottica (Milan)
        "ROST",      // Ross Stores (NASDAQ)
        "VFC",       // VF Corp (NYSE)
        "HM-B.ST",   // H&M (Stockholm)
        "UHR.SW",    // Swatch Group (Swiss)
        "TPR",       // Coach/Tapestry (NYSE) - COH is now TPR
        "ZAL.DE",    // Zalando (German)
        "GPS",       // Gap (NYSE)
        "BBWI",      // L Brands is now Bath & Body Works (NYSE)
        "BRBY.L",    // Burberry (London)
        "1929.HK",   // Chow Tai Fook (Hong Kong)
        "PVH",       // PVH (NYSE)
        "LULU",      // Lululemon (NASDAQ)
        "1913.HK",   // Prada Group (Hong Kong)
        "VIPS",      // Vipshop Holdings (NYSE)
        "NXT.L",     // Next (London)
        "CPRI",      // Michael Kors is now Capri Holdings (NYSE)
        "M",         // Macy's (NYSE)
        "MONC.MI",   // Moncler (Milan)
        "RL",        // Ralph Lauren (NYSE)
        "JWN",       // Nordstrom (NYSE)
        "ASC.L",     // ASOS (London)
        "LREN3.SA",  // Lojas Renner (Brazil)
        "HBI",       // Hanes (NYSE)
        "UAA",       // Under Armour (NYSE)
        "PUM.DE",    // PUMA (German)
        "MKS.L",     // Marks & Spencer (London)
        "SKX",       // Skechers (NYSE)
        "BOSS.DE",   // Hugo Boss (German)
        "GIL.TO",    // Gildan (Toronto)
    ];
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/combined_marketcaps_{}.csv", timestamp);
    let file = std::fs::File::create(&filename)?;
    let mut writer = csv::Writer::from_writer(file);

    // Write headers
    writer.write_record(&[
        "Ticker",
        "Name",
        "Market Cap (Original)",
        "Original Currency",
        "Market Cap (EUR)",
        "Market Cap (USD)",
        "Exchange",
        "Price",
        "Active",
        "Description",
        "Homepage URL",
    ])?;

    // Collect all results first
    let mut results = Vec::new();
    for ticker in tickers {
        println!("Fetching data for {}", ticker);
        match fmp_client.get_details(ticker).await {
            Ok(details) => {
                let original_market_cap = details.market_cap.unwrap_or(0.0);
                let currency = details.currency_symbol.clone().unwrap_or_default();
                let eur_market_cap = convert_currency(original_market_cap, &currency, "EUR");
                let usd_market_cap = convert_currency(original_market_cap, &currency, "USD");
                
                results.push((
                    eur_market_cap,
                    vec![
                        details.ticker,
                        details.name.unwrap_or_default(),
                        original_market_cap.round().to_string(),
                        currency,
                        eur_market_cap.round().to_string(),
                        usd_market_cap.round().to_string(),
                        details.extra.get("exchange").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        details.extra.get("price").map(|v| v.to_string()).unwrap_or_default(),
                        details.active.map(|a| a.to_string()).unwrap_or_default(),
                        details.description.unwrap_or_default(),
                        details.homepage_url.unwrap_or_default(),
                    ]
                ));
                println!("✅ Data collected");
            }
            Err(e) => {
                eprintln!("Error fetching data for {}: {}", ticker, e);
                results.push((
                    0.0,
                    vec![
                        ticker.to_string(),
                        "ERROR".to_string(),
                        "0".to_string(),
                        "".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "".to_string(),
                        "".to_string(),
                        "".to_string(),
                        format!("Error: {}", e),
                        "".to_string(),
                    ]
                ));
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Sort by EUR market cap (highest to lowest)
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Write sorted results to CSV
    for (_, record) in &results {
        writer.write_record(record)?;
    }

    // After writing the CSV, create the visualization
    let stocks: Vec<viz::StockData> = results
        .iter()
        .map(|(_, record)| viz::StockData {
            symbol: record[0].clone(),
            market_cap_eur: record[4].parse::<f64>().unwrap_or(0.0),
            price_change: record[7].parse::<f64>().unwrap_or(0.0),
        })
        .collect();

    let viz_output_path = format!("output/market_heatmap_{}.png", 
        Local::now().format("%Y%m%d_%H%M%S"));
    viz::create_market_heatmap(stocks, &viz_output_path)?;
    
    println!("Market heatmap saved to: {}", viz_output_path);
    println!("📝 CSV file created: {}", filename);
    println!("💶 Results are sorted by market cap in EUR (highest to lowest)");
    Ok(())
}
