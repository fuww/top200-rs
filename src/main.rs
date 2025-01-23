// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod api;
mod config;
mod currencies;
mod db;
mod historical_marketcaps;
mod marketcaps_csv_writer;
mod models;

use anyhow::Result;
use chrono::Datelike;
use clap::{Parser, Subcommand};
use sqlx::sqlite::SqlitePoolOptions;
use std::env;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch historical market cap data for all tickers
    FetchHistoricalMarketCaps {
        /// Start year (inclusive)
        start_year: i32,
        /// End year (inclusive)
        end_year: i32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Create SQLite connection pool
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:top200.db".to_string());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    match &cli.command {
        Some(Commands::FetchHistoricalMarketCaps {
            start_year,
            end_year,
        }) => {
            historical_marketcaps::fetch_historical_market_caps(&pool, *start_year, *end_year)
                .await?;
        }
        None => {
            // Get the current year
            let current_year = chrono::Local::now().year();
            
            // Fetch historical market caps for the current year
            historical_marketcaps::fetch_historical_market_caps(&pool, current_year, current_year)
                .await?;

            // Generate reports
            marketcaps_csv_writer::generate_reports(&pool).await?;
        }
    }

    Ok(())
}
