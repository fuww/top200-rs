use anyhow::Result;
use plotters::prelude::*;

pub struct StockData {
    pub symbol: String,
    pub market_cap_eur: f64,
    pub price_change: f64,
}

pub fn create_market_heatmap(
    stocks: Vec<StockData>,
    output_path: &str,
) -> Result<()> {
    // Use a smaller resolution for better memory usage
    let width = 1280i32;
    let height = 800i32;
    
    let root = BitMapBackend::new(output_path, (width as u32, height as u32))
        .into_drawing_area();
    
    root.fill(&WHITE)?;

    // Draw title
    let title_style = ("sans-serif", 40).into_font().color(&BLACK);
    root.draw_text(
        "Fashion & Luxury Market Cap Heatmap",
        &title_style,
        (width / 2 - 300, 20),
    )?;

    // Sort stocks by market cap
    let mut sorted_stocks = stocks;
    sorted_stocks.sort_by(|a, b| b.market_cap_eur.partial_cmp(&a.market_cap_eur).unwrap());

    // Calculate total market cap for relative sizing
    let total_market_cap: f64 = sorted_stocks.iter().map(|s| s.market_cap_eur).sum();

    // Layout parameters
    let margin = 60i32;
    let title_height = 80i32;
    let usable_width = width - 2 * margin;
    let usable_height = height - title_height - margin;
    let mut current_x = margin;
    let mut current_y = title_height;
    let mut row_height = 40i32; // Minimum row height

    // Calculate initial box sizes
    let boxes: Vec<_> = sorted_stocks.iter().map(|stock| {
        let relative_size = (stock.market_cap_eur / total_market_cap) as f32;
        let box_area = relative_size * (usable_width * usable_height) as f32;
        let box_width = (box_area / row_height as f32) as i32;
        (stock, box_width)
    }).collect();

    // Layout boxes
    for (stock, mut box_width) in boxes {
        // Ensure box width doesn't exceed remaining space
        box_width = box_width.min(usable_width - (current_x - margin));
        
        // Move to next row if needed
        if current_x + box_width > width - margin {
            current_x = margin;
            current_y += row_height;
            
            // Skip if we're out of vertical space
            if current_y + row_height > height - margin {
                break;
            }
        }

        // Determine color based on price change
        let color = if stock.price_change >= 0.0 {
            RGBColor(
                0,
                ((stock.price_change * 20.0) as u8).min(255),
                0
            )
        } else {
            RGBColor(
                ((stock.price_change.abs() * 20.0) as u8).min(255),
                0,
                0
            )
        };

        // Draw rectangle
        let rect = Rectangle::new(
            [(current_x, current_y), (current_x + box_width, current_y + row_height)],
            color.filled(),
        );
        root.draw(&rect)?;

        // Draw border
        let border = Rectangle::new(
            [(current_x, current_y), (current_x + box_width, current_y + row_height)],
            Into::<ShapeStyle>::into(&BLACK).stroke_width(1),
        );
        root.draw(&border)?;

        // Draw text
        let font_size = ((row_height as f32 * 0.3) as i32).min(32).max(12);
        let style = ("sans-serif", font_size).into_font().color(&WHITE);

        // Draw symbol
        root.draw_text(
            &stock.symbol,
            &style,
            (current_x + 5, current_y + 5),
        )?;

        // Draw percentage
        let percentage = format!("{:+.2}%", stock.price_change);
        let percentage_style = ("sans-serif", (font_size * 2 / 3).max(10))
            .into_font()
            .color(&WHITE);
        root.draw_text(
            &percentage,
            &percentage_style,
            (current_x + 5, current_y + font_size + 5),
        )?;

        current_x += box_width;
    }

    root.present()?;
    Ok(())
}
