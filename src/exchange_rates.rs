// SPDX-FileCopyrightText: 2025 Joost van der Laan
// SPDX-License-Identifier: AGPL-3.0-only

use crate::api::FMPClient;
use crate::currencies::insert_forex_rate;
use anyhow::Result;
use chrono::Utc;
use sqlx::sqlite::SqlitePool;

/// Update exchange rates in the database
pub async fn update_exchange_rates(fmp_client: &FMPClient, pool: &SqlitePool) -> Result<()> {
    // Fetch exchange rates
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

    // Store rates in database (use UTC timestamp for consistency)
    let timestamp = Utc::now().timestamp();
    for rate in exchange_rates {
        if let (Some(name), Some(price)) = (rate.name, rate.price) {
            insert_forex_rate(pool, &name, price, price, timestamp).await?;
        }
    }

    println!("✅ Exchange rates updated in database");
    Ok(())
}
