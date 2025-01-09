// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use plotters::prelude::*;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StockData {
    pub symbol: String,
    pub market_cap_eur: f64,
    pub employees: String,
}

struct Rectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn worst_ratio(row: &[(f64, StockData)], width: f64) -> f64 {
    if row.is_empty() {
        return f64::INFINITY;
    }
    let sum: f64 = row.iter().map(|(area, _)| *area).sum::<f64>();
    let height = sum / width;
    row.iter().fold(0.0, |worst, (area, _)| {
        let rect_width = *area / height;
        let ratio = if rect_width > height {
            rect_width / height
        } else {
            height / rect_width
        };
        worst.max(ratio)
    })
}

fn layout_row(
    row: &[(f64, StockData)],
    width: i32,
    x: i32,
    y: i32,
) -> Vec<(Rectangle, StockData)> {
    let _total_area: f64 = row.iter().map(|(area, _)| *area).sum::<f64>();
    let height =
        (row.iter().map(|(area, _)| *area).sum::<f64>() / width as f64) as i32;
    let mut current_x = x;
    let mut result = Vec::new();

    for (area, stock) in row {
        let rect_width = (*area
            / row.iter().map(|(area, _)| *area).sum::<f64>()
            * width as f64) as i32;
        result.push((
            Rectangle {
                x: current_x,
                y,
                width: rect_width,
                height,
            },
            stock.clone(),
        ));
        current_x += rect_width;
    }
    result
}

fn squarify(
    stocks: &[(f64, StockData)],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
) -> Vec<(Rectangle, StockData)> {
    if stocks.is_empty() {
        return Vec::new();
    }

    let _total_area: f64 = stocks.iter().map(|(area, _)| *area).sum::<f64>();
    let shortest_side = width.min(height) as f64;

    let mut current_row = Vec::new();
    let mut i = 0;
    let mut current_ratio = f64::INFINITY;

    while i < stocks.len() {
        let next = stocks[i].clone();
        let mut new_row = current_row.clone();
        new_row.push(next);

        let new_ratio = worst_ratio(&new_row, shortest_side);

        if !current_row.is_empty() && new_ratio > current_ratio {
            let current_row_owned: Vec<(f64, StockData)> =
                current_row.iter().map(|(a, s)| (*a, s.clone())).collect();
            let mut result = if width < height {
                layout_row(&current_row_owned, width, x, y)
            } else {
                layout_row(&current_row_owned, height, y, x)
                    .into_iter()
                    .map(|(rect, stock)| {
                        (
                            Rectangle {
                                x: rect.y,
                                y: rect.x,
                                width: rect.height,
                                height: rect.width,
                            },
                            stock,
                        )
                    })
                    .collect()
            };

            let current_area: f64 =
                current_row.iter().map(|(area, _)| *area).sum::<f64>();
            let remaining_width = if width < height {
                width
            } else {
                height - (current_area / shortest_side) as i32
            };
            let remaining_height = if width < height {
                height - (current_area / shortest_side) as i32
            } else {
                width
            };
            let remaining_x = if width < height {
                x
            } else {
                x + (current_area / shortest_side) as i32
            };
            let remaining_y = if width < height {
                y + (current_area / shortest_side) as i32
            } else {
                y
            };

            result.extend(squarify(
                &stocks[i..]
                    .iter()
                    .map(|(a, s)| (*a, s.clone()))
                    .collect::<Vec<_>>(),
                remaining_width,
                remaining_height,
                remaining_x,
                remaining_y,
            ));
            return result;
        }

        current_row = new_row;
        current_ratio = new_ratio;
        i += 1;
    }

    let current_row_owned: Vec<(f64, StockData)> =
        current_row.iter().map(|(a, s)| (*a, s.clone())).collect();
    if width < height {
        layout_row(&current_row_owned, width, x, y)
    } else {
        layout_row(&current_row_owned, height, y, x)
            .into_iter()
            .map(|(rect, stock)| {
                (
                    Rectangle {
                        x: rect.y,
                        y: rect.x,
                        width: rect.height,
                        height: rect.width,
                    },
                    stock,
                )
            })
            .collect()
    }
}

pub fn create_market_heatmap(
    stocks: Vec<StockData>,
    output_path: &str,
) -> Result<()> {
    let width = 1200i32;
    let height = 1200i32;

    let root = BitMapBackend::new(output_path, (width as u32, height as u32))
        .into_drawing_area();

    root.fill(&WHITE)?;

    // Draw title
    let title_style = ("sans-serif", 40).into_font().color(&BLACK);
    root.draw_text(
        "Fashion & Luxury Market Cap Treemap",
        &title_style,
        (width / 2 - 300, 20),
    )?;

    // Sort stocks by market cap
    let mut sorted_stocks = stocks;
    sorted_stocks.sort_by(|a, b| {
        b.market_cap_eur.partial_cmp(&a.market_cap_eur).unwrap()
    });

    // Calculate total market cap and normalize areas
    let total_market_cap: f64 =
        sorted_stocks.iter().map(|s| s.market_cap_eur).sum();
    let usable_width = width - 100;
    let usable_height = height - 100;
    let total_area = (usable_width * usable_height) as f64;

    let normalized_stocks: Vec<(f64, StockData)> = sorted_stocks
        .iter()
        .map(|stock| {
            let area = (stock.market_cap_eur / total_market_cap) * total_area;
            (area, stock.clone())
        })
        .collect();

    // Generate treemap layout
    let rectangles = squarify(
        &normalized_stocks,
        usable_width,
        usable_height,
        50, // x margin
        80, // y margin (larger to account for title)
    );

    // Draw rectangles
    for (rect, stock) in rectangles {
        // Calculate color based on market cap (green gradient)
        let relative_market_cap =
            stock.market_cap_eur / sorted_stocks[0].market_cap_eur;
        let color = RGBColor(
            (100.0 * (1.0 - relative_market_cap as f32)) as u8,
            (200.0 * relative_market_cap as f32) as u8,
            100,
        );

        // Draw filled rectangle
        let rect_elem = plotters::element::Rectangle::new(
            [
                (rect.x, rect.y),
                (rect.x + rect.width, rect.y + rect.height),
            ],
            color.filled(),
        );
        root.draw(&rect_elem)?;

        // Draw border
        let border = plotters::element::Rectangle::new(
            [
                (rect.x, rect.y),
                (rect.x + rect.width, rect.y + rect.height),
            ],
            Into::<ShapeStyle>::into(BLACK).stroke_width(1),
        );
        root.draw(&border)?;

        // Calculate font size based on rectangle size
        let min_dimension = rect.width.min(rect.height);
        let font_size = ((min_dimension as f32 * 0.2) as i32).clamp(10, 32);
        let text_style =
            ("sans-serif", font_size as u32).into_font().color(&BLACK);

        // Draw text if rectangle is large enough
        if min_dimension > 40 {
            // Draw symbol
            root.draw_text(
                &stock.symbol,
                &text_style,
                (rect.x + 5, rect.y + font_size),
            )?;

            // Format market cap in billions
            let market_cap_b = stock.market_cap_eur / 1_000_000_000.0;
            let market_cap_text = format!("€{:.1}B", market_cap_b);

            // Draw market cap if there's enough vertical space
            if rect.height > font_size * 3 {
                root.draw_text(
                    &market_cap_text,
                    &text_style,
                    (rect.x + 5, rect.y + font_size * 2),
                )?;
            }
        }
    }

    root.present()?;
    Ok(())
}
