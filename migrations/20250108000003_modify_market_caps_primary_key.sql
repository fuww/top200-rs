-- Drop the old table
DROP TABLE IF EXISTS market_caps;

-- Create the new table with a composite primary key
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
