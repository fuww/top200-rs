// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod api;
mod bar_chart;
mod config;
mod currencies;
mod db;
mod details_eu_fmp;
mod details_us_polygon;
mod exchange_rates;
mod marketcaps;
mod models;
mod utils;
mod viz;

use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::sqlite::SqlitePool;
use std::env;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Export US market caps to CSV
    ExportUs,
    /// Export EU market caps to CSV
    ExportEu,
    /// Export combined market caps to CSV
    ExportCombined,
    /// List US market caps
    ListUs,
    /// List EU market caps
    ListEu,
    /// Export exchange rates to CSV
    ExportRates,
    /// Add a currency
    AddCurrency { code: String, name: String },
    /// List currencies
    ListCurrencies,
    /// Generate bar chart of top 100 companies
    GenerateBarChart,
    // /// Generate heatmap
    // GenerateHeatmap,
    // /// List top 100
    // ListTop100,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db".to_string());
    let pool = db::create_db_pool(&db_url).await?;

    match cli.command {
        Some(Commands::ExportUs) => details_us_polygon::export_details_us_csv(&pool).await?,
        Some(Commands::ExportEu) => details_eu_fmp::export_details_eu_csv(&pool).await?,
        Some(Commands::ExportCombined) => {
            // details_us_polygon::export_details_us_csv(&pool).await?;
            // details_eu_fmp::export_details_eu_csv(&pool).await?;
            marketcaps::marketcaps(&pool).await?;
        }
        Some(Commands::ListUs) => details_us_polygon::list_details_us(&pool).await?,
        Some(Commands::ListEu) => details_eu_fmp::list_details_eu(&pool).await?,
        Some(Commands::ExportRates) => {
            let api_key = env::var("FINANCIALMODELINGPREP_API_KEY")
                .expect("FINANCIALMODELINGPREP_API_KEY must be set");
            let fmp_client = api::FMPClient::new(api_key);
            exchange_rates::export_exchange_rates_csv(&fmp_client, &pool).await?;
        }
        Some(Commands::AddCurrency { code, name }) => {
            currencies::insert_currency(&pool, &code, &name).await?;
            println!("Added currency: {} ({})", name, code);
        }
        Some(Commands::ListCurrencies) => {
            let currencies = currencies::list_currencies(&pool).await?;
            for (code, name) in currencies {
                println!("{}: {}", code, name);
            }
        }
        Some(Commands::GenerateBarChart) => {
            async fn generate_bar_chart_handler(pool: &SqlitePool) -> Result<()> {
                bar_chart::generate_bar_chart(pool).await?;
                Ok(())
            }
            generate_bar_chart_handler(&pool).await?;
        }
        // Some(Commands::GenerateHeatmap) => {
        //     marketcaps::generate_heatmap_from_latest()?;
        // }
        // Some(Commands::ListTop100) => {
        //     marketcaps::output_top_100_active()?;
        // }
        None => {
            marketcaps::marketcaps(&pool).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
