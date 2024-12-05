use dioxus::prelude::*;
use std::path::PathBuf;
use csv::{ReaderBuilder, StringRecord};
use std::error::Error;
use std::fs::File;
use rusqlite::{Connection, Result as SqlResult};

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    ticker: String,
    name: String,
    market_cap_eur: f64,
    market_cap_usd: f64,
    exchange: String,
}

#[component]
pub fn App() -> Element {
    let companies = use_state(|| vec![
        Company {
            ticker: "LVMH.PA".to_string(),
            name: "LVMH Moët Hennessy Louis Vuitton".to_string(),
            market_cap_eur: 400.5e9,
            market_cap_usd: 432.8e9,
            exchange: "Euronext Paris".to_string(),
        },
        Company {
            ticker: "NKE".to_string(),
            name: "Nike Inc".to_string(),
            market_cap_eur: 150.2e9,
            market_cap_usd: 162.4e9,
            exchange: "NYSE".to_string(),
        },
        Company {
            ticker: "ADDYY".to_string(),
            name: "Adidas AG".to_string(),
            market_cap_eur: 32.5e9,
            market_cap_usd: 35.1e9,
            exchange: "OTC".to_string(),
        },
    ]);

    rsx! {
        div { class: "container mx-auto p-4",
            h1 { class: "text-3xl font-bold mb-4",
                "Top Fashion Companies by Market Cap"
            }
            
            div { class: "overflow-y-auto max-h-screen",
                ul { class: "space-y-2",
                    companies.iter().map(|company| rsx!(
                        li { 
                            class: "p-4 bg-white rounded-lg shadow hover:shadow-md transition-shadow",
                            div { class: "flex justify-between items-center",
                                div { class: "space-y-1",
                                    div { class: "font-bold", "{company.name} ({company.ticker})" }
                                    div { class: "text-gray-600", "Exchange: {company.exchange}" }
                                }
                                div { class: "text-right space-y-1",
                                    div { format!("€{:.2}B", company.market_cap_eur / 1_000_000_000.0) }
                                    div { class: "text-gray-600", format!("${:.2}B", company.market_cap_usd / 1_000_000_000.0) }
                                }
                            }
                        }
                    ))
                }
            }
        }
    }
}

pub fn init_db() -> SqlResult<()> {
    let conn = Connection::open("companies.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS companies (
            ticker TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            market_cap_eur REAL NOT NULL,
            market_cap_usd REAL NOT NULL,
            exchange TEXT NOT NULL
        )",
        [],
    )?;

    Ok(())
}

pub fn load_csv_to_sqlite() -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("companies.db")?;
    init_db()?;

    let file = File::open("output/combined_marketcaps_20241205_171822.csv")?;
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);

    // Begin transaction for better performance
    let tx = conn.transaction()?;
    
    for result in rdr.records() {
        let record = result?;
        tx.execute(
            "INSERT OR REPLACE INTO companies (ticker, name, market_cap_eur, market_cap_usd, exchange) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &record[0], // ticker
                &record[1], // name
                record[2].parse::<f64>().unwrap_or(0.0).to_string(), // market_cap_eur
                record[3].parse::<f64>().unwrap_or(0.0).to_string(), // market_cap_usd
                &record[4], // exchange
            ],
        )?;
    }

    // Commit the transaction
    tx.commit()?;
    Ok(())
}

pub fn load_companies_from_db() -> Result<Vec<Company>, Box<dyn Error>> {
    let conn = Connection::open("companies.db")?;
    let mut stmt = conn.prepare(
        "SELECT ticker, name, market_cap_eur, market_cap_usd, exchange FROM companies"
    )?;

    let companies = stmt.query_map([], |row| {
        Ok(Company {
            ticker: row.get(0)?,
            name: row.get(1)?,
            market_cap_eur: row.get(2)?,
            market_cap_usd: row.get(3)?,
            exchange: row.get(4)?,
        })
    })?
    .collect::<SqlResult<Vec<Company>>>()?;

    Ok(companies)
}

pub fn load_companies() -> Result<Vec<Company>, Box<dyn Error>> {
    // Try to load from database first
    match load_companies_from_db() {
        Ok(companies) => Ok(companies),
        Err(_) => {
            // If database loading fails, try to load CSV into database and then load from database
            load_csv_to_sqlite()?;
            load_companies_from_db()
        }
    }
}
