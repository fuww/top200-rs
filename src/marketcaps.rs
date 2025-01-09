async fn export_marketcaps(fmp_client: &api::FMPClient) -> Result<()> {
    let config = config::load_config()?;
    let tickers = [config.non_us_tickers, config.us_tickers].concat();

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
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            rate_map.insert(name, price);
        }
    }

    // Convert exchange prefixes to FMP format
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
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
        "Timestamp",
    ])?;

    // Create a rate_map Arc for sharing between tasks
    let rate_map = Arc::new(rate_map);
    let total_tickers = tickers.len();

    // Process tickers in parallel with progress tracking
    let mut results = Vec::new();
    let progress = indicatif::ProgressBar::new(total_tickers as u64);
    progress.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Create chunks of tickers to process in parallel
    // Process 50 tickers at a time to stay well within rate limits
    for chunk in tickers.chunks(50) {
        let chunk_futures = chunk.iter().map(|ticker| {
            let rate_map = rate_map.clone();
            let ticker = ticker.to_string();
            let progress = progress.clone();

            async move {
                let result = match fmp_client.get_details(&ticker, &rate_map).await {
                    Ok(details) => {
                        let original_market_cap = details.market_cap.unwrap_or(0.0);
                        let currency = details.currency_symbol.clone().unwrap_or_default();
                        let eur_market_cap = crate::utils::convert_currency(
                            original_market_cap,
                            &currency,
                            "EUR",
                            &rate_map,
                        );
                        let usd_market_cap = crate::utils::convert_currency(
                            original_market_cap,
                            &currency,
                            "USD",
                            &rate_map,
                        );

                        Some((
                            eur_market_cap,
                            vec![
                                details.ticker,
                                details.name.unwrap_or_default(),
                                original_market_cap.round().to_string(),
                                currency,
                                eur_market_cap.round().to_string(),
                                usd_market_cap.round().to_string(),
                                details
                                    .extra
                                    .get("exchange")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                details
                                    .extra
                                    .get("price")
                                    .map(|v| v.to_string())
                                    .unwrap_or_default(),
                                details.active.map(|a| a.to_string()).unwrap_or_default(),
                                details.description.unwrap_or_default(),
                                details.homepage_url.unwrap_or_default(),
                                details.employees.unwrap_or_default(),
                                details.revenue.map(|r| r.to_string()).unwrap_or_default(),
                                details
                                    .revenue_usd
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details
                                    .working_capital_ratio
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details
                                    .quick_ratio
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details.eps.map(|r| r.to_string()).unwrap_or_default(),
                                details.pe_ratio.map(|r| r.to_string()).unwrap_or_default(),
                                details
                                    .debt_equity_ratio
                                    .map(|r| r.to_string())
                                    .unwrap_or_default(),
                                details.roe.map(|r| r.to_string()).unwrap_or_default(),
                                details.timestamp.unwrap_or_default(),
                            ],
                        ))
                    }
                    Err(e) => {
                        eprintln!("Error fetching data for {}: {}", ticker, e);
                        None
                    }
                };
                progress.inc(1);
                result
            }
        });

        // Wait for the current chunk to complete
        let chunk_results: Vec<_> = futures::future::join_all(chunk_futures).await;
        results.extend(chunk_results.into_iter().flatten());
    }

    progress.finish_with_message("Data collection complete");

    // Sort by market cap (EUR)
    results.sort_by(|(a_cap, _): &(f64, Vec<String>), (b_cap, _)| {
        b_cap
            .partial_cmp(a_cap)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Write all results
    for (_, record) in &results {
        writer.write_record(record)?;
    }
    writer.flush()?;
    println!("✅ Combined market caps written to: {}", filename);

    // Filter active tickers and get top 100
    let top_100_results: Vec<(f64, Vec<String>)> = results
        .iter()
        .filter(|(_, record)| record[8] == "true") // Active column
        .take(100)
        .map(|(cap, record)| (*cap, record.clone()))
        .collect();

    // Generate top 100 CSV
    let top_100_filename = format!("output/top_100_active_{}.csv", timestamp);
    let top_100_file = std::fs::File::create(&top_100_filename)?;
    let mut top_100_writer = csv::Writer::from_writer(top_100_file);

    // Write headers
    top_100_writer.write_record(&[
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
        "Employees",
        "Revenue",
        "Revenue (USD)",
        "Working Capital Ratio",
        "Quick Ratio",
        "EPS",
        "P/E Ratio",
        "D/E Ratio",
        "ROE",
        "Timestamp",
    ])?;

    // Write top 100 records
    for (_, record) in &top_100_results {
        top_100_writer.write_record(record)?;
    }
    top_100_writer.flush()?;
    println!("✅ Top 100 active tickers written to: {}", top_100_filename);

    // Generate market heatmap from top 100
    generate_market_heatmap(&top_100_results, "output/market_heatmap.png")?;
    println!("✅ Market heatmap generated from top 100 active tickers");

    Ok(())
}