// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use plotters::prelude::*;

use crate::viz::StockData;

pub fn create_market_cap_histogram(stocks: &[StockData], output_path: &str) -> Result<()> {
    let root = BitMapBackend::new(output_path, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let market_caps: Vec<f64> = stocks.iter().map(|s| s.market_cap_eur).collect();
    let max_cap = market_caps.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let min_cap = market_caps.iter().fold(f64::INFINITY, |a, &b| a.min(b));

    // Create 20 bins for the histogram
    const NUM_BINS: usize = 20;
    let bin_size = (max_cap - min_cap) / NUM_BINS as f64;
    let mut bins = vec![0; NUM_BINS];

    // Count values in each bin
    for &cap in &market_caps {
        let bin_index = ((cap - min_cap) / bin_size).floor() as usize;
        let bin_index = bin_index.min(NUM_BINS - 1); // Handle edge case for max value
        bins[bin_index] += 1;
    }

    let max_count = *bins.iter().max().unwrap_or(&0);

    let mut chart = ChartBuilder::on(&root)
        .caption("Market Cap Distribution", ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0..NUM_BINS, 0..max_count + 1)?;

    chart
        .configure_mesh()
        .x_desc("Market Cap (EUR)")
        .y_desc("Number of Companies")
        .x_label_formatter(&|x| {
            let value = min_cap + (*x as f64 * bin_size);
            format!("{:.1}B", value / 1e9) // Convert to billions
        })
        .draw()?;

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(BLUE.filled())
            .margin(0)
            .data(bins.iter().enumerate().map(|(i, &count)| (i, count))),
    )?;

    root.present()?;
    Ok(())
}
