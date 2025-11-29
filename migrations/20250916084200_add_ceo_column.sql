-- Add CEO column to ticker_details table
-- Note: SQLite doesn't support IF NOT EXISTS for ALTER TABLE ADD COLUMN
-- If this migration has already been applied, it will fail with an error
ALTER TABLE ticker_details ADD COLUMN ceo TEXT;
