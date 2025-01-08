-- Add forex_rates table
CREATE TABLE IF NOT EXISTS forex_rates (
    symbol TEXT NOT NULL,
    ask REAL NOT NULL,
    bid REAL NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (symbol, timestamp)
);
