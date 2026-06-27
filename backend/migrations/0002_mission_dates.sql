-- Alter missions table to add explicit start_date for scheduling
ALTER TABLE missions ADD COLUMN IF NOT EXISTS start_date TIMESTAMPTZ NOT NULL DEFAULT NOW();
