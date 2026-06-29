-- 1. Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- 2. Drop existing telemetry table (data is reconstructed live by simulator)
DROP TABLE IF EXISTS telemetry CASCADE;

-- 3. Re-create telemetry table with composite primary key (partition column must be in PK)
CREATE TABLE telemetry (
    id BIGSERIAL,
    satellite_id UUID NOT NULL REFERENCES satellites(id) ON DELETE CASCADE,
    battery_level DOUBLE PRECISION NOT NULL,
    battery_temp DOUBLE PRECISION NOT NULL,
    solar_power DOUBLE PRECISION NOT NULL,
    velocity DOUBLE PRECISION NOT NULL,
    altitude DOUBLE PRECISION NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
);

-- 4. Convert telemetry table to TimescaleDB hypertable
SELECT create_hypertable('telemetry', 'created_at');

-- 5. Re-create index on hypertable
CREATE INDEX idx_telemetry_satellite_time ON telemetry (satellite_id, created_at DESC);
