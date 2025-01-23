-- Create currencies table
CREATE TABLE IF NOT EXISTS currencies (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create forex_rates table
CREATE TABLE IF NOT EXISTS forex_rates (
    symbol TEXT NOT NULL,
    ask REAL NOT NULL,
    bid REAL NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (symbol, timestamp)
);

-- Create market_caps table
CREATE TABLE IF NOT EXISTS market_caps (
    ticker TEXT NOT NULL,
    name TEXT NOT NULL,
    market_cap_original DECIMAL,
    original_currency TEXT,
    market_cap_eur DECIMAL,
    market_cap_usd DECIMAL,
    exchange TEXT,
    price DECIMAL,
    active BOOLEAN,
    description TEXT,
    homepage_url TEXT,
    employees INTEGER,
    revenue DECIMAL,
    revenue_usd DECIMAL,
    working_capital_ratio DECIMAL,
    quick_ratio DECIMAL,
    eps DECIMAL,
    pe_ratio DECIMAL,
    de_ratio DECIMAL,
    roe DECIMAL,
    timestamp DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (ticker, timestamp)
);
