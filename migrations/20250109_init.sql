-- Create currencies table
CREATE TABLE IF NOT EXISTS currencies (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create forex_rates table
CREATE TABLE IF NOT EXISTS forex_rates (
    symbol TEXT NOT NULL,
    bid REAL NOT NULL,
    ask REAL NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (symbol, timestamp)
);

-- Create market_caps table
CREATE TABLE IF NOT EXISTS market_caps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ticker TEXT NOT NULL,
    name TEXT,
    market_cap_original REAL,
    original_currency TEXT,
    market_cap_eur REAL,
    market_cap_usd REAL,
    exchange TEXT,
    price REAL,
    active BOOLEAN,
    description TEXT,
    homepage_url TEXT,
    employees TEXT,
    revenue REAL,
    revenue_usd REAL,
    working_capital_ratio REAL,
    quick_ratio REAL,
    eps REAL,
    pe_ratio REAL,
    de_ratio REAL,
    roe REAL,
    timestamp TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(ticker)
);
