-- Add migration script here
CREATE TABLE IF NOT EXISTS ticker_details (
    ticker TEXT PRIMARY KEY,
    description TEXT,
    homepage_url TEXT,
    employees INTEGER,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Remove these columns from market_caps table
ALTER TABLE market_caps DROP COLUMN description;
ALTER TABLE market_caps DROP COLUMN homepage_url;
