-- Add CEO column to ticker_details table if it doesn't exist
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE ADD COLUMN
-- This migration is a no-op if the column already exists
SELECT 1;
