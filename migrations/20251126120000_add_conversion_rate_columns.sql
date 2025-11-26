-- Add conversion rate columns to market_caps table
-- These store the exchange rates used for EUR and USD conversions

ALTER TABLE market_caps ADD COLUMN eur_rate DECIMAL;
ALTER TABLE market_caps ADD COLUMN usd_rate DECIMAL;
